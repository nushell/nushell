use std::{
    ffi::OsStr,
    sync::{Arc, Mutex},
};

use nu_protocol::{PluginGcConfig, PluginIdentity, RegisteredPlugin, ShellError};

use super::{create_command, gc::PluginGc, make_plugin_interface, PluginInterface, PluginSource};

/// A box that can keep a plugin that was spawned persistent for further uses. The plugin may or
/// may not be currently running. [`.get()`] gets the currently running plugin, or spawns it if it's
/// not running.
///
/// Note: used in the parser, not for plugin authors
#[doc(hidden)]
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
    /// Garbage collector config
    gc_config: PluginGcConfig,
}

#[derive(Debug)]
struct RunningPlugin {
    /// Process ID of the running plugin
    pid: u32,
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
                gc_config,
            }),
        }
    }

    /// Get the plugin interface of the running plugin, or spawn it if it's not currently running.
    ///
    /// Will call `envs` to get environment variables to spawn the plugin if the plugin needs to be
    /// spawned.
    pub(crate) fn get<E, K, V>(
        self: Arc<Self>,
        envs: impl FnOnce() -> Result<E, ShellError>,
    ) -> Result<PluginInterface, ShellError>
    where
        E: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
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
            // Try to spawn, and then store the spawned plugin if we were successful.
            //
            // We hold the lock the whole time to prevent others from trying to spawn and ending
            // up with duplicate plugins
            let new_running = self.clone().spawn(envs()?, &mutable.gc_config)?;
            let interface = new_running.interface.clone();
            mutable.running = Some(new_running);
            Ok(interface)
        }
    }

    /// Run the plugin command, then set up and return [`RunningPlugin`].
    fn spawn(
        self: Arc<Self>,
        envs: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
        gc_config: &PluginGcConfig,
    ) -> Result<RunningPlugin, ShellError> {
        let source_file = self.identity.filename();
        let mut plugin_cmd = create_command(source_file, self.identity.shell());

        // We need the current environment variables for `python` based plugins
        // Or we'll likely have a problem when a plugin is implemented in a virtual Python environment.
        plugin_cmd.envs(envs);

        let program_name = plugin_cmd.get_program().to_os_string().into_string();

        // Run the plugin command
        let child = plugin_cmd.spawn().map_err(|err| {
            let error_msg = match err.kind() {
                std::io::ErrorKind::NotFound => match program_name {
                    Ok(prog_name) => {
                        format!("Can't find {prog_name}, please make sure that {prog_name} is in PATH.")
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
        let gc = PluginGc::new(gc_config.clone(), &self)?;

        let pid = child.id();
        let interface =
            make_plugin_interface(child, Arc::new(PluginSource::new(&self)), Some(gc.clone()))?;

        Ok(RunningPlugin { pid, interface, gc })
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
            .and_then(|r| r.running.as_ref().map(|r| r.pid))
    }

    fn stop(&self) -> Result<(), ShellError> {
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
        Ok(())
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
}
