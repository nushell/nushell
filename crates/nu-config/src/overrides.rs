use std::path::{Path, PathBuf};

/// CLI flags that affect *where* configuration files are found.
///
/// Only path-resolution flags belong here. Flags that control *whether* config
/// files are loaded (e.g. `--no-config-file`) stay in `NushellCliArgs`.
///
/// Relative paths are resolved against the provided cwd in
/// [`CliOverrides::from_path_strings`] — that is the single place absolute-ization
/// of CLI path overrides happens.
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

impl CliOverrides {
    /// Build overrides from optional CLI path strings, resolving any relative
    /// path against `cwd`.
    ///
    /// This is the **only** place CLI config path absolute-ization should live.
    pub fn from_path_strings(
        config_home: Option<&str>,
        config_file: Option<&str>,
        env_file: Option<&str>,
        #[cfg(feature = "plugin")] plugin_file: Option<&str>,
        cwd: &Path,
    ) -> Self {
        Self {
            config_home: config_home.map(|s| absolutize_cli_path(s, cwd)),
            config_file: config_file.map(|s| absolutize_cli_path(s, cwd)),
            env_file: env_file.map(|s| absolutize_cli_path(s, cwd)),
            #[cfg(feature = "plugin")]
            plugin_file: plugin_file.map(|s| absolutize_cli_path(s, cwd)),
        }
    }
}

/// Resolve a CLI path string against `cwd` when it is relative.
fn absolutize_cli_path(path: &str, cwd: &Path) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() { p } else { cwd.join(p) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_join_cwd() {
        let cwd = PathBuf::from("/tmp/work");
        let cli = CliOverrides::from_path_strings(
            Some("cfg-home"),
            Some("config.nu"),
            Some("env.nu"),
            #[cfg(feature = "plugin")]
            Some("plugin.msgpackz"),
            &cwd,
        );
        assert_eq!(
            cli.config_home.as_deref(),
            Some(Path::new("/tmp/work/cfg-home"))
        );
        assert_eq!(
            cli.config_file.as_deref(),
            Some(Path::new("/tmp/work/config.nu"))
        );
        assert_eq!(cli.env_file.as_deref(), Some(Path::new("/tmp/work/env.nu")));
        #[cfg(feature = "plugin")]
        assert_eq!(
            cli.plugin_file.as_deref(),
            Some(Path::new("/tmp/work/plugin.msgpackz"))
        );
    }

    #[test]
    fn absolute_paths_kept() {
        let cwd = PathBuf::from("/tmp/work");
        let abs = if cfg!(windows) {
            r"C:\nushell\config.nu"
        } else {
            "/etc/nushell/config.nu"
        };
        let cli = CliOverrides::from_path_strings(
            None,
            Some(abs),
            None,
            #[cfg(feature = "plugin")]
            None,
            &cwd,
        );
        assert_eq!(cli.config_file.as_deref(), Some(Path::new(abs)));
    }
}
