use std::path::PathBuf;

/// CLI flags that affect *where* configuration files are found.
///
/// Only path-resolution flags belong here.  Flags that control *whether* config
/// files are loaded (e.g. `--no-config-file`) stay in `NushellCliArgs`.
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    /// `--config-home <path>` — override the entire nushell config directory
    pub config_home: Option<PathBuf>,

    /// `--config <file>` — override `config.nu` path
    pub config_file: Option<PathBuf>,

    /// `--env-config <file>` — override `env.nu` path
    pub env_file: Option<PathBuf>,

    /// `--plugin-config <file>` — override plugin registry file path
    #[cfg(feature = "plugin")]
    pub plugin_file: Option<PathBuf>,
}
