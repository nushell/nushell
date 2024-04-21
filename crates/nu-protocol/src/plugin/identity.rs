use std::path::{Path, PathBuf};

use crate::{ParseError, ShellError, Spanned};

/// Error when an invalid plugin filename was encountered.
#[derive(Debug, Clone)]
pub struct InvalidPluginFilename(PathBuf);

impl std::fmt::Display for InvalidPluginFilename {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid plugin filename")
    }
}

impl From<Spanned<InvalidPluginFilename>> for ParseError {
    fn from(error: Spanned<InvalidPluginFilename>) -> ParseError {
        ParseError::LabeledError(
            "Invalid plugin filename".into(),
            "must start with `nu_plugin_`".into(),
            error.span,
        )
    }
}

impl From<Spanned<InvalidPluginFilename>> for ShellError {
    fn from(error: Spanned<InvalidPluginFilename>) -> ShellError {
        ShellError::GenericError {
            error: format!("Invalid plugin filename: {}", error.item.0.display()),
            msg: "not a valid plugin filename".into(),
            span: Some(error.span),
            help: Some("valid Nushell plugin filenames must start with `nu_plugin_`".into()),
            inner: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginIdentity {
    /// The filename used to start the plugin
    filename: PathBuf,
    /// The shell used to start the plugin, if required
    shell: Option<PathBuf>,
    /// The friendly name of the plugin (e.g. `inc` for `C:\nu_plugin_inc.exe`)
    name: String,
}

impl PluginIdentity {
    /// Create a new plugin identity from a path to plugin executable and shell option.
    ///
    /// The `filename` must be an absolute path. Canonicalize before trying to construct the
    /// [`PluginIdentity`].
    pub fn new(
        filename: impl Into<PathBuf>,
        shell: Option<PathBuf>,
    ) -> Result<PluginIdentity, InvalidPluginFilename> {
        let filename: PathBuf = filename.into();

        // Must pass absolute path.
        if filename.is_relative() {
            return Err(InvalidPluginFilename(filename));
        }

        let name = filename
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .and_then(|stem| stem.strip_prefix("nu_plugin_").map(|s| s.to_owned()))
            .ok_or_else(|| InvalidPluginFilename(filename.clone()))?;

        Ok(PluginIdentity {
            filename,
            shell,
            name,
        })
    }

    /// The filename of the plugin executable.
    pub fn filename(&self) -> &Path {
        &self.filename
    }

    /// The shell command used by the plugin.
    pub fn shell(&self) -> Option<&Path> {
        self.shell.as_deref()
    }

    /// The name of the plugin, determined by the part of the filename after `nu_plugin_` excluding
    /// the extension.
    ///
    /// - `C:\nu_plugin_inc.exe` becomes `inc`
    /// - `/home/nu/.cargo/bin/nu_plugin_inc` becomes `inc`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Create a fake identity for testing.
    #[cfg(windows)]
    #[doc(hidden)]
    pub fn new_fake(name: &str) -> PluginIdentity {
        PluginIdentity::new(format!(r"C:\fake\path\nu_plugin_{name}.exe"), None)
            .expect("fake plugin identity path is invalid")
    }

    /// Create a fake identity for testing.
    #[cfg(not(windows))]
    #[doc(hidden)]
    pub fn new_fake(name: &str) -> PluginIdentity {
        PluginIdentity::new(format!(r"/fake/path/nu_plugin_{name}"), None)
            .expect("fake plugin identity path is invalid")
    }

    /// A command that could be used to add the plugin, for suggesting in errors.
    pub fn add_command(&self) -> String {
        if let Some(shell) = self.shell() {
            format!(
                "plugin add --shell '{}' '{}'",
                shell.display(),
                self.filename().display(),
            )
        } else {
            format!("plugin add '{}'", self.filename().display())
        }
    }

    /// A command that could be used to reload the plugin, for suggesting in errors.
    pub fn use_command(&self) -> String {
        format!("plugin use '{}'", self.name())
    }
}

#[test]
fn parses_name_from_path() {
    assert_eq!("test", PluginIdentity::new_fake("test").name());
    assert_eq!("test_2", PluginIdentity::new_fake("test_2").name());
    let absolute_path = if cfg!(windows) {
        r"C:\path\to\nu_plugin_foo.sh"
    } else {
        "/path/to/nu_plugin_foo.sh"
    };
    assert_eq!(
        "foo",
        PluginIdentity::new(absolute_path, Some("sh".into()))
            .expect("should be valid")
            .name()
    );
    // Relative paths should be invalid
    PluginIdentity::new("nu_plugin_foo.sh", Some("sh".into())).expect_err("should be invalid");
    PluginIdentity::new("other", None).expect_err("should be invalid");
    PluginIdentity::new("", None).expect_err("should be invalid");
}
