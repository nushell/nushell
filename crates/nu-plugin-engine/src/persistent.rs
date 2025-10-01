use crate::{
    PluginGc,
    init::{create_command, make_plugin_interface},
};

use super::{PluginInterface, PluginSource};
use nu_plugin_core::CommunicationMode;
use nu_protocol::{
    HandlerGuard, Handlers, PluginGcConfig, PluginIdentity, PluginMetadata, RegisteredPlugin,
    ShellError,
    engine::{EngineState, Stack},
    shell_error::io::IoError,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// A box that can keep a plugin that was spawned persistent for further uses. The plugin may or
/// may not be currently running. [`.get()`] gets the currently running plugin, or spawns it if it's
/// not running.
#[derive(Debug)]
pub struct PersistentPlugin {
    /// Identity (filename, shell, name) of the plugin
    identity: PluginIdentity,
    /// Mutable state
    mutable: Mutex<MutableState>,
}

/// The mutable state for the persistent plugin. This should all be behind one lock to prevent lock
/// order problems.
#[derive(Debug)]
struct MutableState {
    /// Reference to the plugin if running
    running: Option<RunningPlugin>,
    /// Metadata for the plugin, e.g. version.
    metadata: Option<PluginMetadata>,
    /// Plugin's preferred communication mode (if known)
    preferred_mode: Option<PreferredCommunicationMode>,
    /// Garbage collector config
    gc_config: PluginGcConfig,
    /// RAII guard for this plugin's signal handler
    signal_guard: Option<HandlerGuard>,
}

#[derive(Debug, Clone, Copy)]
enum PreferredCommunicationMode {
    Stdio,
    #[cfg(feature = "local-socket")]
    LocalSocket,
}

#[derive(Debug)]
struct RunningPlugin {
    /// Interface (which can be cloned) to the running plugin
    interface: PluginInterface,
    /// Garbage collector for the plugin
    gc: PluginGc,
}

impl PersistentPlugin {
    /// Create a new persistent plugin. The plugin will not be spawned immediately.
    pub fn new(identity: PluginIdentity, gc_config: PluginGcConfig) -> PersistentPlugin {
        PersistentPlugin {
            identity,
            mutable: Mutex::new(MutableState {
                running: None,
                metadata: None,
                preferred_mode: None,
                gc_config,
                signal_guard: None,
            }),
        }
    }

    /// Get the plugin interface of the running plugin, or spawn it if it's not currently running.
    ///
    /// Will call `envs` to get environment variables to spawn the plugin if the plugin needs to be
    /// spawned.
    pub fn get(
        self: Arc<Self>,
        envs: impl FnOnce() -> Result<HashMap<String, String>, ShellError>,
    ) -> Result<PluginInterface, ShellError> {
        let mut mutable = self.mutable.lock().map_err(|_| ShellError::NushellFailed {
            msg: format!(
                "plugin `{}` mutex poisoned, probably panic during spawn",
                self.identity.name()
            ),
        })?;

        if let Some(ref running) = mutable.running {
            // It exists, so just clone the interface
            Ok(running.interface.clone())
        } else {
            // Try to spawn. On success, `mutable.running` should have been set to the new running
            // plugin by `spawn()` so we just then need to clone the interface from there.
            //
            // We hold the lock the whole time to prevent others from trying to spawn and ending
            // up with duplicate plugins
            //
            // TODO: We should probably store the envs somewhere, in case we have to launch without
            // envs (e.g. from a custom value)
            let envs = envs()?;
            let result = self.clone().spawn(&envs, &mut mutable);

            // Check if we were using an alternate communication mode and may need to fall back to
            // stdio.
            if result.is_err()
                && !matches!(
                    mutable.preferred_mode,
                    Some(PreferredCommunicationMode::Stdio)
                )
            {
                log::warn!(
                    "{}: Trying again with stdio communication because mode {:?} failed with {result:?}",
                    self.identity.name(),
                    mutable.preferred_mode
                );
                // Reset to stdio and try again, but this time don't catch any error
                mutable.preferred_mode = Some(PreferredCommunicationMode::Stdio);
                self.clone().spawn(&envs, &mut mutable)?;
            }

            Ok(mutable
                .running
                .as_ref()
                .ok_or_else(|| ShellError::NushellFailed {
                    msg: "spawn() succeeded but didn't set interface".into(),
                })?
                .interface
                .clone())
        }
    }

    /// Run the plugin command, then set up and set `mutable.running` to the new running plugin.
    fn spawn(
        self: Arc<Self>,
        envs: &HashMap<String, String>,
        mutable: &mut MutableState,
    ) -> Result<(), ShellError> {
        // Make sure `running` is set to None to begin
        if let Some(running) = mutable.running.take() {
            // Stop the GC if there was a running plugin
            running.gc.stop_tracking();
        }

        let source_file = self.identity.filename();

        // Determine the mode to use based on the preferred mode
        let mode = match mutable.preferred_mode {
            // If not set, we try stdio first and then might retry if another mode is supported
            Some(PreferredCommunicationMode::Stdio) | None => CommunicationMode::Stdio,
            // Local socket only if enabled
            #[cfg(feature = "local-socket")]
            Some(PreferredCommunicationMode::LocalSocket) => {
                CommunicationMode::local_socket(source_file)
            }
        };

        let mut plugin_cmd = create_command(source_file, self.identity.shell(), &mode);

        // We need the current environment variables for `python` based plugins
        // Or we'll likely have a problem when a plugin is implemented in a virtual Python environment.
        plugin_cmd.envs(envs);

        let program_name = plugin_cmd.get_program().to_os_string().into_string();

        // Before running the command, prepare communication
        let comm = mode.serve()?;

        // Run the plugin command
        let child = plugin_cmd.spawn().map_err(|err| {
            let error_msg = match err.kind() {
                std::io::ErrorKind::NotFound => match program_name {
                    Ok(prog_name) => {
                        format!(
                            "Can't find {prog_name}, please make sure that {prog_name} is in PATH."
                        )
                    }
                    _ => {
                        format!("Error spawning child process: {err}")
                    }
                },
                _ => {
                    format!("Error spawning child process: {err}")
                }
            };
            ShellError::PluginFailedToLoad { msg: error_msg }
        })?;

        // Start the plugin garbage collector
        let gc = PluginGc::new(mutable.gc_config.clone(), &self).map_err(|err| {
            IoError::new_internal(err, "Could not start plugin gc", nu_protocol::location!())
        })?;

        let pid = child.id();
        let interface = make_plugin_interface(
            child,
            comm,
            Arc::new(PluginSource::new(self.clone())),
            Some(pid),
            Some(gc.clone()),
        )?;

        // If our current preferred mode is None, check to see if the plugin might support another
        // mode. If so, retry spawn() with that mode
        #[cfg(feature = "local-socket")]
        if mutable.preferred_mode.is_none()
            && interface
                .protocol_info()?
                .supports_feature(&nu_plugin_protocol::Feature::LocalSocket)
        {
            log::trace!(
                "{}: Attempting to upgrade to local socket mode",
                self.identity.name()
            );
            // Stop the GC we just created from tracking so that we don't accidentally try to
            // stop the new plugin
            gc.stop_tracking();
            // Set the mode and try again
            mutable.preferred_mode = Some(PreferredCommunicationMode::LocalSocket);
            return self.spawn(envs, mutable);
        }

        mutable.running = Some(RunningPlugin { interface, gc });
        Ok(())
    }

    fn stop_internal(&self, reset: bool) -> Result<(), ShellError> {
        let mut mutable = self.mutable.lock().map_err(|_| ShellError::NushellFailed {
            msg: format!(
                "plugin `{}` mutable mutex poisoned, probably panic during spawn",
                self.identity.name()
            ),
        })?;

        // If the plugin is running, stop its GC, so that the GC doesn't accidentally try to stop
        // a future plugin
        if let Some(ref running) = mutable.running {
            running.gc.stop_tracking();
        }

        // We don't try to kill the process or anything, we just drop the RunningPlugin. It should
        // exit soon after
        mutable.running = None;

        // If this is a reset, we should also reset other learned attributes like preferred_mode
        if reset {
            mutable.preferred_mode = None;
        }
        Ok(())
    }
}

impl RegisteredPlugin for PersistentPlugin {
    fn identity(&self) -> &PluginIdentity {
        &self.identity
    }

    fn is_running(&self) -> bool {
        // If the lock is poisoned, we return false here. That may not be correct, but this is a
        // failure state anyway that would be noticed at some point
        self.mutable
            .lock()
            .map(|m| m.running.is_some())
            .unwrap_or(false)
    }

    fn pid(&self) -> Option<u32> {
        // Again, we return None for a poisoned lock.
        self.mutable
            .lock()
            .ok()
            .and_then(|r| r.running.as_ref().and_then(|r| r.interface.pid()))
    }

    fn stop(&self) -> Result<(), ShellError> {
        self.stop_internal(false)
    }

    fn reset(&self) -> Result<(), ShellError> {
        self.stop_internal(true)
    }

    fn metadata(&self) -> Option<PluginMetadata> {
        self.mutable.lock().ok().and_then(|m| m.metadata.clone())
    }

    fn set_metadata(&self, metadata: Option<PluginMetadata>) {
        if let Ok(mut mutable) = self.mutable.lock() {
            mutable.metadata = metadata;
        }
    }

    fn set_gc_config(&self, gc_config: &PluginGcConfig) {
        if let Ok(mut mutable) = self.mutable.lock() {
            // Save the new config for future calls
            mutable.gc_config = gc_config.clone();

            // If the plugin is already running, propagate the config change to the running GC
            if let Some(gc) = mutable.running.as_ref().map(|running| running.gc.clone()) {
                // We don't want to get caught holding the lock
                drop(mutable);
                gc.set_config(gc_config.clone());
                gc.flush();
            }
        }
    }

    fn as_any(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
        self
    }

    fn configure_signal_handler(self: Arc<Self>, handlers: &Handlers) -> Result<(), ShellError> {
        let guard = {
            // We take a weakref to the plugin so that we don't create a cycle to the
            // RAII guard that will be stored on the plugin.
            let plugin = Arc::downgrade(&self);
            handlers.register(Box::new(move |action| {
                // write a signal packet through the PluginInterface if the plugin is alive and
                // running
                if let Some(plugin) = plugin.upgrade()
                    && let Ok(mutable) = plugin.mutable.lock()
                    && let Some(ref running) = mutable.running
                {
                    let _ = running.interface.signal(action);
                }
            }))?
        };

        if let Ok(mut mutable) = self.mutable.lock() {
            mutable.signal_guard = Some(guard);
        }

        Ok(())
    }
}

/// Anything that can produce a plugin interface.
pub trait GetPlugin: RegisteredPlugin {
    /// Retrieve or spawn a [`PluginInterface`]. The `context` may be used for determining
    /// environment variables to launch the plugin with.
    fn get_plugin(
        self: Arc<Self>,
        context: Option<(&EngineState, &mut Stack)>,
    ) -> Result<PluginInterface, ShellError>;
}

impl GetPlugin for PersistentPlugin {
    fn get_plugin(
        self: Arc<Self>,
        mut context: Option<(&EngineState, &mut Stack)>,
    ) -> Result<PluginInterface, ShellError> {
        self.get(|| {
            // Get envs from the context if provided.
            let envs = context
                .as_mut()
                .map(|(engine_state, stack)| {
                    // We need the current environment variables for `python` based plugins. Or
                    // we'll likely have a problem when a plugin is implemented in a virtual Python
                    // environment.
                    let stack = &mut stack.start_collect_value();
                    nu_engine::env::env_to_strings(engine_state, stack)
                })
                .transpose()?;

            Ok(envs.unwrap_or_default())
        })
    }
}
