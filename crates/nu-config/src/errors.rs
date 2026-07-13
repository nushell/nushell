use std::path::PathBuf;

/// Errors that can occur during config-path resolution.
///
/// These are deliberately **not** `ShellError` — this crate sits below
/// `nu-protocol` in the dependency graph. Callers convert to `ShellError` at
/// the boundary.
///
/// Non-fatal XDG issues are **not** errors — they are [`ConfigWarning`]s.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    /// No config directory could be resolved (no XDG env var and the platform
    /// fallback returned `None`).
    #[error("Could not determine a config directory")]
    ConfigDirNotFound,

    /// No home directory could be found via the platform home-dir lookup.
    #[error("Could not determine the home directory")]
    NoHomeDir,
}

/// Non-fatal warnings produced during config-path resolution.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum ConfigWarning {
    /// `$XDG_CONFIG_HOME` was set to a value that was ignored (relative path,
    /// empty string) and the platform default was used instead.
    #[display(
        "$env.XDG_CONFIG_HOME ({xdg}) is set to a non-absolute path, \
        using default config directory instead: {}", 
        resolved.display()
    )]
    XdgConfigIgnored { xdg: String, resolved: PathBuf },

    /// The old config directory (platform default) has files but the new
    /// XDG_CONFIG_HOME directory is empty — the user may have forgotten to
    /// migrate.
    #[display(
        "WARNING: XDG_CONFIG_HOME has been set but {} is empty.\n\
        Nushell will not move your configuration files from {}", 
        new.display(), 
        old.display()
    )]
    OldConfigDirHasFiles { old: PathBuf, new: PathBuf },
}
