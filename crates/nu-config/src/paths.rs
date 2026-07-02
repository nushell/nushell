use std::path::PathBuf;

/// All resolved configuration directories and file paths for Nushell.
///
/// Every path here is the *final* answer after applying the full resolution
/// chain: CLI overrides → XDG env vars → platform defaults.
///
/// # `$nu` constant
///
/// Every field in this struct maps to one or more fields in the `$nu` constant.
/// `create_nu_constant()` in `nu-protocol` reads from `engine_state.config_dirs`
/// instead of calling ad-hoc path-resolution functions.
#[derive(Debug, Clone)]
pub struct NushellConfigDirs {
    /// The nushell config directory (e.g. `~/.config/nushell`).
    /// Maps to `$nu.default-config-dir`.
    pub config_home: PathBuf,

    /// Path to `config.nu` — either the CLI override (`--config`) or
    /// `config_home/config.nu`.  Maps to `$nu.config-path`.
    pub config_file: PathBuf,

    /// Path to `env.nu` — either the CLI override (`--env-config`) or
    /// `config_home/env.nu`.  Maps to `$nu.env-path`.
    pub env_file: PathBuf,

    /// The nushell data directory (e.g. `~/.local/share/nushell`).
    /// Maps to `$nu.data-dir`.
    pub data_home: PathBuf,

    /// The nushell cache directory (e.g. `~/.cache/nushell`).
    /// Maps to `$nu.cache-dir`.
    pub cache_home: PathBuf,

    /// The user's home directory.  Maps to `$nu.home-dir`.
    pub home_dir: PathBuf,

    /// Vendor autoload directories — directories from which Nushell
    /// automatically loads `.nu` files at startup.  These come from
    /// `XDG_DATA_DIRS`, platform-specific paths, and `$NU_VENDOR_AUTOLOAD_DIR`.
    /// Maps to `$nu.vendor-autoload-dirs`.
    pub vendor_autoload_dirs: Vec<PathBuf>,

    /// User autoload directories — `config_home/autoload`.
    /// Maps to `$nu.user-autoload-dirs`.
    pub user_autoload_dirs: Vec<PathBuf>,

    /// Path to the plugin registry file — either the CLI override
    /// (`--plugin-config`) or `config_home/plugin.msgpackz`.
    /// Maps to `$nu.plugin-path`.
    #[cfg(feature = "plugin")]
    pub plugin_file: PathBuf,
}

impl NushellConfigDirs {
    /// Create an empty/inert instance for use before `resolve_paths()` has
    /// been called (e.g. in `EngineState::new()`).
    ///
    /// All paths are empty.  Call `resolve_paths()` before accessing `$nu`.
    pub fn empty() -> Self {
        Self {
            config_home: PathBuf::new(),
            config_file: PathBuf::new(),
            env_file: PathBuf::new(),
            data_home: PathBuf::new(),
            cache_home: PathBuf::new(),
            home_dir: PathBuf::new(),
            vendor_autoload_dirs: Vec::new(),
            user_autoload_dirs: Vec::new(),
            #[cfg(feature = "plugin")]
            plugin_file: PathBuf::new(),
        }
    }
}
