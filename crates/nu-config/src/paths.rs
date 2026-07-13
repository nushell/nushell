//! Resolved config directory layout and path types.
//!
//! [`NushellConfigDirs`] is the single source of truth after
//! [`crate::resolve_paths`] runs. File paths that may be CLI-overridden use
//! [`ConfigPath`] so origin (default vs override) stays in the type system.

use std::path::{Path, PathBuf};

/// A resolved config-related file path that records whether it came from a
/// CLI override or from the default location under `config_home`.
///
/// Origin is part of the type so loaders can decide whether missing files
/// should error (override) or be scaffolded (default) without a parallel bool.
///
/// # Examples
///
/// ```
/// use nu_config::ConfigPath;
/// use std::path::PathBuf;
///
/// let default = ConfigPath::Default(PathBuf::from("/home/me/.config/nushell/config.nu"));
/// assert!(!default.is_override());
///
/// let r#override = ConfigPath::Override(PathBuf::from("/tmp/custom.nu"));
/// assert!(r#override.is_override());
/// assert_eq!(r#override.as_path(), PathBuf::from("/tmp/custom.nu"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigPath {
    /// Path derived from the resolved config home (e.g. `config_home/config.nu`).
    /// First-run scaffolding is allowed when the file is missing.
    Default(PathBuf),
    /// Path supplied by a CLI flag (`--config`, `--env-config`, `--plugin-config`).
    /// A missing file is an error; do not scaffold.
    Override(PathBuf),
}

impl ConfigPath {
    /// The concrete filesystem path.
    pub fn as_path(&self) -> &Path {
        match self {
            Self::Default(path) | Self::Override(path) => path,
        }
    }

    /// Owned copy of the concrete filesystem path.
    pub fn to_path_buf(&self) -> PathBuf {
        self.as_path().to_path_buf()
    }

    /// Consume and return the concrete filesystem path.
    pub fn into_path_buf(self) -> PathBuf {
        match self {
            Self::Default(path) | Self::Override(path) => path,
        }
    }

    /// Whether this path came from a CLI override.
    pub fn is_override(&self) -> bool {
        matches!(self, Self::Override(_))
    }

    /// Empty default path — used only for inert pre-resolve state
    /// ([`NushellConfigDirs::empty`]).
    pub fn empty_default() -> Self {
        Self::Default(PathBuf::new())
    }
}

impl AsRef<Path> for ConfigPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl std::fmt::Display for ConfigPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_path().display().fmt(f)
    }
}

/// All resolved configuration directories and file paths for Nushell.
///
/// Every path here is the *final* answer after applying the full resolution
/// chain: CLI overrides → XDG env vars → platform defaults.
///
/// This is the **single source of truth** for where config lives. Downstream
/// code (config-file loading, `$nu` constant generation, history backends,
/// plugin registry, etc.) must read from this struct instead of re-resolving.
///
/// # `$nu` constant
///
/// Most fields map to `$nu.*` members. See field docs for the mapping.
/// `create_nu_constant()` in `nu-protocol` reads from `engine_state.config_dirs`.
///
/// # Empty state
///
/// [`NushellConfigDirs::empty`] is used only before `resolve_paths` runs (or
/// when resolution fails). Check [`Self::is_resolved`] before treating paths
/// as meaningful.
#[derive(Debug, Clone)]
pub struct NushellConfigDirs {
    /// The nushell config directory (e.g. `~/.config/nushell`).
    /// Maps to `$nu.default-config-dir`.
    pub config_home: PathBuf,

    /// Path to `config.nu` — either a CLI override (`--config`) or
    /// `config_home/config.nu`. Maps to `$nu.config-path`.
    pub config_file: ConfigPath,

    /// Path to `env.nu` — either a CLI override (`--env-config`) or
    /// `config_home/env.nu`. Maps to `$nu.env-path`.
    pub env_file: ConfigPath,

    /// The nushell data directory (e.g. `~/.local/share/nushell`).
    /// Maps to `$nu.data-dir`.
    pub data_home: PathBuf,

    /// The nushell cache directory (e.g. `~/.cache/nushell`).
    /// Maps to `$nu.cache-dir`.
    pub cache_home: PathBuf,

    /// The user's home directory. Maps to `$nu.home-dir`.
    pub home_dir: PathBuf,

    /// Vendor autoload directories — directories from which Nushell
    /// automatically loads `.nu` files at startup. These come from
    /// `XDG_DATA_DIRS`, platform-specific paths, and `$NU_VENDOR_AUTOLOAD_DIR`.
    /// Maps to `$nu.vendor-autoload-dirs`.
    ///
    /// Order matters: files are evaluated in list order, so later entries
    /// override earlier ones. On Unix, `XDG_DATA_DIRS` is reversed so that
    /// earlier entries in the env var win (XDG precedence).
    pub vendor_autoload_dirs: Vec<PathBuf>,

    /// User autoload directories — `config_home/autoload`.
    /// Maps to `$nu.user-autoload-dirs`.
    pub user_autoload_dirs: Vec<PathBuf>,

    /// Path to the plugin registry file — either a CLI override
    /// (`--plugin-config`) or `config_home/plugin.msgpackz`.
    /// Maps to `$nu.plugin-path`.
    #[cfg(feature = "plugin")]
    pub plugin_file: ConfigPath,
}

impl NushellConfigDirs {
    /// Create an empty/inert instance for use before `resolve_paths()` has
    /// been called (e.g. in `EngineState::new()`).
    ///
    /// All paths are empty. Call `resolve_paths()` before accessing `$nu`.
    pub fn empty() -> Self {
        Self {
            config_home: PathBuf::new(),
            config_file: ConfigPath::empty_default(),
            env_file: ConfigPath::empty_default(),
            data_home: PathBuf::new(),
            cache_home: PathBuf::new(),
            home_dir: PathBuf::new(),
            vendor_autoload_dirs: Vec::new(),
            user_autoload_dirs: Vec::new(),
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::empty_default(),
        }
    }

    /// Whether resolution produced usable paths (config home is non-empty).
    pub fn is_resolved(&self) -> bool {
        !self.config_home.as_os_str().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_default_is_not_override() {
        let path = ConfigPath::Default(PathBuf::from("/cfg/config.nu"));
        assert!(!path.is_override());
        assert_eq!(path.as_path(), Path::new("/cfg/config.nu"));
        assert_eq!(path.to_path_buf(), PathBuf::from("/cfg/config.nu"));
    }

    #[test]
    fn config_path_override_is_override() {
        let path = ConfigPath::Override(PathBuf::from("/tmp/x.nu"));
        assert!(path.is_override());
        assert_eq!(path.to_string(), "/tmp/x.nu");
    }

    #[test]
    fn empty_dirs_are_unresolved() {
        assert!(!NushellConfigDirs::empty().is_resolved());
    }

    #[test]
    fn into_path_buf_consumes_either_variant() {
        assert_eq!(
            ConfigPath::Default(PathBuf::from("a")).into_path_buf(),
            PathBuf::from("a")
        );
        assert_eq!(
            ConfigPath::Override(PathBuf::from("b")).into_path_buf(),
            PathBuf::from("b")
        );
    }
}
