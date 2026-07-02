use std::fmt;
use std::path::PathBuf;

/// Errors that can occur during config-path resolution.
///
/// These are deliberately **not** `ShellError` — this crate sits below
/// `nu-protocol` in the dependency graph.  Callers convert to `ShellError` at
/// the boundary.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// No config directory could be resolved (no XDG env var and the platform
    /// fallback returned `None`).
    ConfigDirNotFound,

    /// `$XDG_CONFIG_HOME` was set to a value that could not be used (relative
    /// path, empty string, etc.) and the platform default was used instead.
    InvalidXdgConfig { xdg: String, resolved: PathBuf },

    /// No home directory could be found via `dirs::home_dir()`.
    NoHomeDir,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigDirNotFound => {
                write!(f, "Could not determine a config directory")
            }
            Self::InvalidXdgConfig { xdg, resolved } => {
                write!(
                    f,
                    "$env.XDG_CONFIG_HOME ({xdg}) is invalid, using default config directory instead: {}",
                    resolved.display(),
                )
            }
            Self::NoHomeDir => {
                write!(f, "Could not determine the home directory")
            }
        }
    }
}

/// Non-fatal warnings produced during config-path resolution.
#[derive(Debug, Clone)]
pub enum ConfigWarning {
    /// `$XDG_CONFIG_HOME` was set to a value that was ignored (relative path,
    /// empty string) and the platform default was used instead.
    XdgConfigIgnored { xdg: String, resolved: PathBuf },

    /// The old config directory (platform default) has files but the new
    /// XDG_CONFIG_HOME directory is empty — the user may have forgotten to
    /// migrate.
    OldConfigDirHasFiles { old: PathBuf, new: PathBuf },
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XdgConfigIgnored { xdg, resolved } => {
                write!(
                    f,
                    "$env.XDG_CONFIG_HOME ({xdg}) is set to a non-absolute path, using default config directory instead: {}",
                    resolved.display(),
                )
            }
            Self::OldConfigDirHasFiles { old, new } => {
                writeln!(
                    f,
                    "WARNING: XDG_CONFIG_HOME has been set but {} is empty.",
                    new.display(),
                )?;
                write!(
                    f,
                    "Nushell will not move your configuration files from {}",
                    old.display(),
                )
            }
        }
    }
}
