use std::{
    io::{BufReader, BufWriter},
    path::Path,
    process::Child,
    sync::{Arc, Mutex},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use nu_plugin_core::{
    CommunicationMode, EncodingType, InterfaceManager, PreparedServerCommunication,
    ServerCommunicationIo,
};
use nu_protocol::{
    engine::StateWorkingSet, report_error_new, PluginCacheFile, PluginCacheItem,
    PluginCacheItemData, PluginIdentity, RegisteredPlugin, ShellError, Span,
};

use crate::{
    PersistentPlugin, PluginDeclaration, PluginGc, PluginInterface, PluginInterfaceManager,
    PluginSource,
};

pub(crate) const OUTPUT_BUFFER_SIZE: usize = 8192;

pub fn create_command(
    path: &Path,
    mut shell: Option<&Path>,
    mode: &CommunicationMode,
) -> std::process::Command {
    log::trace!("Starting plugin: {path:?}, shell = {shell:?}, mode = {mode:?}");

    let mut shell_args = vec![];

    if shell.is_none() {
        // We only have to do this for things that are not executable by Rust's Command API on
        // Windows. They do handle bat/cmd files for us, helpfully.
        //
        // Also include anything that wouldn't be executable with a shebang, like JAR files.
        shell = match path.extension().and_then(|e| e.to_str()) {
            Some("sh") => {
                if cfg!(unix) {
                    // We don't want to override what might be in the shebang if this is Unix, since
                    // some scripts will have a shebang specifying bash even if they're .sh
                    None
                } else {
                    Some(Path::new("sh"))
                }
            }
            Some("nu") => {
                shell_args.push("--stdin");
                Some(Path::new("nu"))
            }
            Some("py") => Some(Path::new("python")),
            Some("rb") => Some(Path::new("ruby")),
            Some("jar") => {
                shell_args.push("-jar");
                Some(Path::new("java"))
            }
            _ => None,
        };
    }

    let mut process = if let Some(shell) = shell {
        let mut process = std::process::Command::new(shell);
        process.args(shell_args);
        process.arg(path);

        process
    } else {
        std::process::Command::new(path)
    };

    process.args(mode.args());

    // Setup I/O according to the communication mode
    mode.setup_command_io(&mut process);

    // The plugin should be run in a new process group to prevent Ctrl-C from stopping it
    #[cfg(unix)]
    process.process_group(0);
    #[cfg(windows)]
    process.creation_flags(windows::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP.0);

    // In order to make bugs with improper use of filesystem without getting the engine current
    // directory more obvious, the plugin always starts in the directory of its executable
    if let Some(dirname) = path.parent() {
        process.current_dir(dirname);
    }

    process
}

pub fn make_plugin_interface(
    mut child: Child,
    comm: PreparedServerCommunication,
    source: Arc<PluginSource>,
    pid: Option<u32>,
    gc: Option<PluginGc>,
) -> Result<PluginInterface, ShellError> {
    match comm.connect(&mut child)? {
        ServerCommunicationIo::Stdio(stdin, stdout) => make_plugin_interface_with_streams(
            stdout,
            stdin,
            move || {
                let _ = child.wait();
            },
            source,
            pid,
            gc,
        ),
        #[cfg(feature = "local-socket")]
        ServerCommunicationIo::LocalSocket { read_out, write_in } => {
            make_plugin_interface_with_streams(
                read_out,
                write_in,
                move || {
                    let _ = child.wait();
                },
                source,
                pid,
                gc,
            )
        }
    }
}

pub fn make_plugin_interface_with_streams(
    mut reader: impl std::io::Read + Send + 'static,
    writer: impl std::io::Write + Send + 'static,
    after_close: impl FnOnce() + Send + 'static,
    source: Arc<PluginSource>,
    pid: Option<u32>,
    gc: Option<PluginGc>,
) -> Result<PluginInterface, ShellError> {
    let encoder = get_plugin_encoding(&mut reader)?;

    let reader = BufReader::with_capacity(OUTPUT_BUFFER_SIZE, reader);
    let writer = BufWriter::with_capacity(OUTPUT_BUFFER_SIZE, writer);

    let mut manager =
        PluginInterfaceManager::new(source.clone(), pid, (Mutex::new(writer), encoder));
    manager.set_garbage_collector(gc);

    let interface = manager.get_interface();
    interface.hello()?;

    // Spawn the reader on a new thread. We need to be able to read messages at the same time that
    // we write, because we are expected to be able to handle multiple messages coming in from the
    // plugin at any time, including stream messages like `Drop`.
    std::thread::Builder::new()
        .name(format!(
            "plugin interface reader ({})",
            source.identity.name()
        ))
        .spawn(move || {
            if let Err(err) = manager.consume_all((reader, encoder)) {
                log::warn!("Error in PluginInterfaceManager: {err}");
            }
            // If the loop has ended, drop the manager so everyone disconnects and then run
            // after_close
            drop(manager);
            after_close();
        })
        .map_err(|err| ShellError::PluginFailedToLoad {
            msg: format!("Failed to spawn thread for plugin: {err}"),
        })?;

    Ok(interface)
}

pub fn get_plugin_encoding(
    child_stdout: &mut impl std::io::Read,
) -> Result<EncodingType, ShellError> {
    let mut length_buf = [0u8; 1];
    child_stdout
        .read_exact(&mut length_buf)
        .map_err(|e| ShellError::PluginFailedToLoad {
            msg: format!("unable to get encoding from plugin: {e}"),
        })?;

    let mut buf = vec![0u8; length_buf[0] as usize];
    child_stdout
        .read_exact(&mut buf)
        .map_err(|e| ShellError::PluginFailedToLoad {
            msg: format!("unable to get encoding from plugin: {e}"),
        })?;

    EncodingType::try_from_bytes(&buf).ok_or_else(|| {
        let encoding_for_debug = String::from_utf8_lossy(&buf);
        ShellError::PluginFailedToLoad {
            msg: format!("get unsupported plugin encoding: {encoding_for_debug}"),
        }
    })
}
/// Load the definitions from the plugin file into the engine state
pub fn load_plugin_file(
    working_set: &mut StateWorkingSet,
    plugin_cache_file: &PluginCacheFile,
    span: Option<Span>,
) {
    for plugin in &plugin_cache_file.plugins {
        // Any errors encountered should just be logged.
        if let Err(err) = load_plugin_cache_item(working_set, plugin, span) {
            report_error_new(working_set.permanent_state, &err)
        }
    }
}

/// Load a definition from the plugin file into the engine state
pub fn load_plugin_cache_item(
    working_set: &mut StateWorkingSet,
    plugin: &PluginCacheItem,
    span: Option<Span>,
) -> Result<Arc<PersistentPlugin>, ShellError> {
    let identity =
        PluginIdentity::new(plugin.filename.clone(), plugin.shell.clone()).map_err(|_| {
            ShellError::GenericError {
                error: "Invalid plugin filename in plugin cache file".into(),
                msg: "loaded from here".into(),
                span,
                help: Some(format!(
                    "the filename for `{}` is not a valid nushell plugin: {}",
                    plugin.name,
                    plugin.filename.display()
                )),
                inner: vec![],
            }
        })?;

    match &plugin.data {
        PluginCacheItemData::Valid { commands } => {
            let plugin = add_plugin_to_working_set(working_set, &identity)?;

            // Ensure that the plugin is reset. We're going to load new signatures, so we want to
            // make sure the running plugin reflects those new signatures, and it's possible that it
            // doesn't.
            plugin.reset()?;

            // Create the declarations from the commands
            for signature in commands {
                let decl = PluginDeclaration::new(plugin.clone(), signature.clone());
                working_set.add_decl(Box::new(decl));
            }
            Ok(plugin)
        }
        PluginCacheItemData::Invalid => Err(ShellError::PluginCacheDataInvalid {
            plugin_name: identity.name().to_owned(),
            span,
            add_command: identity.add_command(),
        }),
    }
}

/// Find [`PersistentPlugin`] with the given `identity` in the `working_set`, or construct it
/// if it doesn't exist.
///
/// The garbage collection config is always found and set in either case.
pub fn add_plugin_to_working_set(
    working_set: &mut StateWorkingSet,
    identity: &PluginIdentity,
) -> Result<Arc<PersistentPlugin>, ShellError> {
    // Find garbage collection config for the plugin
    let gc_config = working_set
        .get_config()
        .plugin_gc
        .get(identity.name())
        .clone();

    // Add it to / get it from the working set
    let plugin = working_set.find_or_create_plugin(identity, || {
        Arc::new(PersistentPlugin::new(identity.clone(), gc_config.clone()))
    });

    plugin.set_gc_config(&gc_config);

    // Downcast the plugin to `PersistentPlugin` - we generally expect this to succeed.
    // The trait object only exists so that nu-protocol can contain plugins without knowing
    // anything about their implementation, but we only use `PersistentPlugin` in practice.
    plugin
        .as_any()
        .downcast()
        .map_err(|_| ShellError::NushellFailed {
            msg: "encountered unexpected RegisteredPlugin type".into(),
        })
}
