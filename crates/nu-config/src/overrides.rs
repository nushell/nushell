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
    ///
    /// Resolution is **logical** (not realpath): tilde is expanded via the
    /// user's home directory, relative segments are joined to `cwd`, and `.` /
    /// `..` are stripped lexically. Symlinks in the path are **not** followed.
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

/// Resolve a CLI path to a logical absolute path against `cwd`.
///
/// Uses [`nu_path::expand_path_with`]: expands a leading `~`, joins relative
/// paths to `cwd`, and lexically normalizes `.` / `..`. Does **not**
/// canonicalize or follow symlinks — so `~/.config/nushell` becomes
/// `$HOME/.config/nushell` even when `$HOME` itself is a symlink chain.
fn absolutize_cli_path(path: &str, cwd: &Path) -> PathBuf {
    nu_path::expand_path_with(path, cwd, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Component;

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

    #[test]
    fn dot_config_home_is_cwd_without_trailing_curdir() {
        let cwd = PathBuf::from("/tmp/work");
        let cli = CliOverrides::from_path_strings(
            Some("."),
            None,
            None,
            #[cfg(feature = "plugin")]
            None,
            &cwd,
        );
        assert_eq!(cli.config_home.as_deref(), Some(cwd.as_path()));
        assert!(
            !cli.config_home
                .as_ref()
                .unwrap()
                .components()
                .any(|c| matches!(c, Component::CurDir))
        );
    }

    #[test]
    fn relative_dot_slash_subdir_normalized() {
        let cwd = PathBuf::from("/tmp/work");
        let cli = CliOverrides::from_path_strings(
            Some("./subdir"),
            Some("./cfg.nu"),
            None,
            #[cfg(feature = "plugin")]
            None,
            &cwd,
        );
        assert_eq!(
            cli.config_home.as_deref(),
            Some(Path::new("/tmp/work/subdir"))
        );
        assert_eq!(
            cli.config_file.as_deref(),
            Some(Path::new("/tmp/work/cfg.nu"))
        );
    }

    #[test]
    fn parent_dir_components_resolved_lexically() {
        let cwd = PathBuf::from("/tmp/work/nested");
        assert_eq!(
            absolutize_cli_path("../sibling", &cwd),
            PathBuf::from("/tmp/work/sibling")
        );
    }

    #[test]
    fn tilde_config_home_expands_to_user_home_without_cwd_join() {
        let Some(home) = dirs::home_dir() else {
            return; // environment without a home dir — skip
        };
        let cwd = PathBuf::from("/tmp/unrelated/cwd");
        let cli = CliOverrides::from_path_strings(
            Some("~/.config/nushell"),
            None,
            None,
            #[cfg(feature = "plugin")]
            None,
            &cwd,
        );
        let expected = home.join(".config").join("nushell");
        assert_eq!(
            cli.config_home.as_deref(),
            Some(expected.as_path()),
            "tilde should expand via $HOME, not join under cwd"
        );
        // Must remain the logical home path (no accidental cwd prefix).
        assert!(
            !cli.config_home.as_ref().unwrap().starts_with(&cwd),
            "must not resolve under cwd"
        );
    }

    #[test]
    fn bare_tilde_expands_to_home() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let cwd = PathBuf::from("/tmp/unrelated");
        assert_eq!(absolutize_cli_path("~", &cwd), home);
    }
}
