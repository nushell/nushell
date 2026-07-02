use crate::env_access::EnvAccess;
use crate::errors::{ConfigError, ConfigWarning};
use crate::overrides::CliOverrides;
use crate::paths::NushellConfigDirs;
use std::path::{Path, PathBuf};

/// One-shot entry point for resolving all Nushell configuration paths.
///
/// Call this once at startup and store the result in `EngineState.config_dirs`.
/// Every downstream consumer (`create_nu_constant`, config-file loading,
/// history backend, etc.) reads from that struct instead of re-resolving.
///
/// # Resolution order
///
/// For each path, the priority is:
/// 1. CLI override (`CliOverrides`)
/// 2. Corresponding XDG environment variable
/// 3. Platform default via the `dirs` crate
///
/// # Errors
///
/// Returns [`ConfigError`] if no config directory or home directory can be
/// determined.  Non-fatal warnings (e.g. an empty XDG config dir) are returned
/// in the [`ConfigWarning`] vec.
pub fn resolve_paths(
    env: &dyn EnvAccess,
    cli: &CliOverrides,
) -> Result<(NushellConfigDirs, Vec<ConfigWarning>), ConfigError> {
    let mut warnings = Vec::new();

    // ── config_home ──────────────────────────────────────────────────────
    let (config_home, xdg_was_absolute) = resolve_config_home_with_source(env, cli)?;

    // ── config_file / env_file ───────────────────────────────────────────
    let config_file = cli
        .config_file
        .clone()
        .unwrap_or_else(|| config_home.join("config.nu"));

    let env_file = cli
        .env_file
        .clone()
        .unwrap_or_else(|| config_home.join("env.nu"));

    // ── data_home ────────────────────────────────────────────────────────
    let data_home = resolve_xdg_base(env, "XDG_DATA_HOME", dirs::data_dir)
        .unwrap_or_else(|| config_home.join("data"))
        .join("nushell");

    // ── cache_home ───────────────────────────────────────────────────────
    let cache_home = resolve_xdg_base(env, "XDG_CACHE_HOME", dirs::cache_dir)
        .unwrap_or_else(|| config_home.join("cache"))
        .join("nushell");

    // ── home_dir ─────────────────────────────────────────────────────────
    let home_dir = dirs::home_dir().ok_or(ConfigError::NoHomeDir)?;

    // ── vendor_autoload_dirs ─────────────────────────────────────────────
    let vendor_autoload_dirs = resolve_vendor_autoload_dirs(env);

    // ── user_autoload_dirs ───────────────────────────────────────────────
    let user_autoload_dirs = vec![config_home.join("autoload")];

    // ── plugin_file ──────────────────────────────────────────────────────
    #[cfg(feature = "plugin")]
    let plugin_file = cli
        .plugin_file
        .clone()
        .unwrap_or_else(|| config_home.join("plugin.msgpackz"));

    // ── XDG validation (moved from main.rs) ──────────────────────────────
    // If the user set XDG_CONFIG_HOME but the platform default was used
    // (because the value was empty or non-absolute), emit a warning.
    if let Ok(xdg_raw) = env.var("XDG_CONFIG_HOME") {
        if !xdg_raw.is_empty() && !xdg_was_absolute {
            warnings.push(ConfigWarning::XdgConfigIgnored {
                xdg: xdg_raw,
                resolved: config_home.clone(),
            });
        }

        // If XDG_CONFIG_HOME was used but the old dir still has files while
        // the new one is empty, warn about the migration.
        if xdg_was_absolute && let Some(old_config) = dirs::config_dir().map(|p| p.join("nushell"))
        {
            let new_config_empty = config_home
                .read_dir()
                .map_or(true, |mut dir| dir.next().is_none());
            let old_config_empty = old_config
                .read_dir()
                .map_or(true, |mut dir| dir.next().is_none());
            if !old_config_empty && new_config_empty {
                warnings.push(ConfigWarning::OldConfigDirHasFiles {
                    old: old_config,
                    new: config_home.clone(),
                });
            }
        }
    }

    Ok((
        NushellConfigDirs {
            config_home: config_home.clone(),
            config_file,
            env_file,
            data_home,
            cache_home,
            home_dir,
            vendor_autoload_dirs,
            user_autoload_dirs,
            #[cfg(feature = "plugin")]
            plugin_file,
        },
        warnings,
    ))
}

// ─── Internal helpers ─────────────────────────────────────────────────────

/// Resolve the nushell config directory and track whether an absolute
/// XDG_CONFIG_HOME was the source.
fn resolve_config_home_with_source(
    env: &dyn EnvAccess,
    cli: &CliOverrides,
) -> Result<(PathBuf, bool), ConfigError> {
    // 1. CLI override takes highest priority.
    if let Some(ref home) = cli.config_home {
        return Ok((home.clone(), false));
    }

    // 2. XDG_CONFIG_HOME env var.
    if let Ok(xdg) = env.var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
        && Path::new(&xdg).is_absolute()
    {
        let mut home = PathBuf::from(xdg);
        home.push("nushell");
        return Ok((home, true));
    }

    // 3. Platform default.
    let base = dirs::config_dir().ok_or(ConfigError::ConfigDirNotFound)?;
    Ok((base.join("nushell"), false))
}

/// Resolve an XDG base directory (CONFIG, DATA, CACHE) with fallback.
fn resolve_xdg_base(
    env: &dyn EnvAccess,
    var_name: &str,
    fallback: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Ok(val) = env.var(var_name)
        && !val.is_empty()
        && Path::new(&val).is_absolute()
    {
        return Some(PathBuf::from(val));
    }
    fallback()
}

/// Convenience function that returns the nushell config home directory
/// using the real system environment and no CLI overrides.
///
/// This is useful for code paths that don't have access to an `EngineState`
/// but still need the config directory (e.g. `HistoryConfig::file_path()`).
pub fn config_home() -> Option<std::path::PathBuf> {
    resolve_config_home_with_source(&crate::env_access::SystemEnv, &CliOverrides::default())
        .ok()
        .map(|(path, _)| path)
}

/// Resolve vendor autoload directories.
///
/// This logic was moved from `nu-protocol/src/eval_const.rs` /
/// `get_vendor_autoload_dirs()` so that all path resolution is in one place.
fn resolve_vendor_autoload_dirs(env: &dyn EnvAccess) -> Vec<PathBuf> {
    let into_autoload_path = |mut path: PathBuf| {
        path.push("nushell");
        path.push("vendor");
        path.push("autoload");
        path
    };

    let mut dirs = Vec::new();

    let mut append = |path: PathBuf| {
        if !dirs.contains(&path) {
            dirs.push(path);
        }
    };

    #[cfg(target_os = "macos")]
    std::iter::once("/Library/Application Support")
        .map(PathBuf::from)
        .map(&into_autoload_path)
        .for_each(&mut append);

    #[cfg(unix)]
    {
        let data_dirs = env.var("XDG_DATA_DIRS").unwrap_or_else(|_| {
            option_env!("PREFIX").map_or_else(
                || "/usr/local/share/:/usr/share/".to_string(),
                |prefix| {
                    if prefix.ends_with("local") {
                        format!("{prefix}/share")
                    } else {
                        format!("{prefix}/local/share:{prefix}/share")
                    }
                },
            )
        });
        for dir in data_dirs.split(':') {
            append(into_autoload_path(PathBuf::from(dir)));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(path) = env.var("ProgramData") {
            append(into_autoload_path(PathBuf::from(path)));
        }
    }

    if let Some(path) = option_env!("NU_VENDOR_AUTOLOAD_DIR") {
        append(PathBuf::from(path));
    }

    if let Some(data_dir) = resolve_xdg_base(env, "XDG_DATA_HOME", dirs::data_dir) {
        append(into_autoload_path(data_dir));
    }

    if let Ok(path) = env.var("NU_VENDOR_AUTOLOAD_DIR")
        && !path.is_empty()
    {
        append(PathBuf::from(path));
    }

    dirs
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::env_access::TestEnv;

    /// Helper: an absolute path for use in tests, on any platform.
    fn abs_path(components: &[&str]) -> PathBuf {
        let mut p = std::env::temp_dir();
        for c in components {
            p.push(c);
        }
        p
    }

    #[test]
    fn test_resolve_paths_uses_xdg_config_home() {
        let xdg_base = abs_path(&["xdg-test"]);
        let xdg_str = xdg_base.to_string_lossy().to_string();

        let mut vars = std::collections::HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), xdg_str.clone());
        let env = TestEnv::new(vars);
        let (dirs, _warnings) =
            resolve_paths(&env, &CliOverrides::default()).expect("resolve should succeed");

        let expected_home = xdg_base.join("nushell");
        assert_eq!(dirs.config_home, expected_home);
        assert_eq!(dirs.config_file, expected_home.join("config.nu"));
        assert_eq!(dirs.env_file, expected_home.join("env.nu"));
    }

    #[test]
    fn test_resolve_paths_config_home_override() {
        let ignored = abs_path(&["xdg-ignored"]).to_string_lossy().to_string();
        let override_path = abs_path(&["my-override"]);

        let mut vars = std::collections::HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), ignored);
        let env = TestEnv::new(vars);
        let cli = CliOverrides {
            config_home: Some(override_path.clone()),
            ..Default::default()
        };
        let (dirs, _warnings) = resolve_paths(&env, &cli).expect("resolve should succeed");
        assert_eq!(dirs.config_home, override_path);
    }

    #[test]
    fn test_resolve_paths_cli_config_file_override() {
        let custom_root = abs_path(&["custom", "path"]);

        let env = TestEnv::new(std::collections::HashMap::new());
        let cli = CliOverrides {
            config_file: Some(custom_root.join("config.nu")),
            env_file: Some(custom_root.join("env.nu")),
            ..Default::default()
        };
        let (dirs, _warnings) =
            resolve_paths(&env, &cli).expect("resolve should succeed");
        assert_eq!(dirs.config_file, custom_root.join("config.nu"));
        assert_eq!(dirs.env_file, custom_root.join("env.nu"));
    }

    #[test]
    fn test_resolve_paths_empty_xdg_falls_back() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), "".into());
        let env = TestEnv::new(vars);
        // Should not error — empty XDG_CONFIG_HOME is ignored
        let _ = resolve_paths(&env, &CliOverrides::default()).expect("empty XDG should fall back");
    }

    #[test]
    fn test_resolve_paths_relative_xdg_ignored() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), "relative/path".into());
        let env = TestEnv::new(vars);
        // Should not error — relative XDG is ignored, platform default used
        let _ =
            resolve_paths(&env, &CliOverrides::default()).expect("relative XDG should be ignored");
    }
}
