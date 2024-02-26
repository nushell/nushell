use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};

use nu_protocol::ShellError;

use super::{create_command, make_plugin_interface, PluginInterface};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginIdentity {
    /// The filename used to start the plugin
    pub(crate) filename: PathBuf,
    /// The shell used to start the plugin, if required
    pub(crate) shell: Option<PathBuf>,
    /// The friendly name of the plugin (e.g. `inc` for `C:\nu_plugin_inc.exe`)
    pub(crate) plugin_name: String,
}

impl PluginIdentity {
    pub(crate) fn new(filename: impl Into<PathBuf>, shell: Option<PathBuf>) -> PluginIdentity {
        let filename = filename.into();
        // `C:\nu_plugin_inc.exe` becomes `inc`
        // `/home/nu/.cargo/bin/nu_plugin_inc` becomes `inc`
        // any other path, including if it doesn't start with nu_plugin_, becomes
        // `<invalid plugin name>`
        let plugin_name = filename
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .and_then(|stem| stem.strip_prefix("nu_plugin_").map(|s| s.to_owned()))
            .unwrap_or_else(|| {
                log::warn!(
                    "filename `{}` is not a valid plugin name, must start with nu_plugin_",
                    filename.display()
                );
                "<invalid plugin name>".into()
            });
        PluginIdentity {
            filename,
            shell,
            plugin_name,
        }
    }

    #[cfg(all(test, windows))]
    pub(crate) fn new_fake(name: &str) -> Arc<PluginIdentity> {
        Arc::new(PluginIdentity::new(
            format!(r"C:\fake\path\nu_plugin_{name}.exe"),
            None,
        ))
    }

    #[cfg(all(test, not(windows)))]
    pub(crate) fn new_fake(name: &str) -> Arc<PluginIdentity> {
        Arc::new(PluginIdentity::new(
            format!(r"/fake/path/nu_plugin_{name}"),
            None,
        ))
    }

    /// Run the plugin command stored in this [`PluginIdentity`], then set up and return the
    /// [`PluginInterface`] attached to it.
    pub(crate) fn spawn(
        self: Arc<Self>,
        envs: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> Result<PluginInterface, ShellError> {
        let source_file = Path::new(&self.filename);
        let mut plugin_cmd = create_command(source_file, self.shell.as_deref());

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

        make_plugin_interface(child, self)
    }
}

#[test]
fn parses_name_from_path() {
    assert_eq!("test", PluginIdentity::new_fake("test").plugin_name);
    assert_eq!(
        "<invalid plugin name>",
        PluginIdentity::new("other", None).plugin_name
    );
    assert_eq!(
        "<invalid plugin name>",
        PluginIdentity::new("", None).plugin_name
    );
}
