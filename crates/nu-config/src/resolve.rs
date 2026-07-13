use crate::env_access::EnvAccess;
use crate::errors::{ConfigError, ConfigWarning};
use crate::overrides::CliOverrides;
use crate::paths::{ConfigPath, NushellConfigDirs};
use std::ffi::OsString;
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
/// 1. CLI override ([`CliOverrides`])
/// 2. Corresponding XDG environment variable
/// 3. Platform default via the [`EnvAccess`] seam
///
/// # Errors
///
/// Returns [`ConfigError`] if no config directory or home directory can be
/// determined. Non-fatal warnings (e.g. an empty XDG config dir) are returned
/// in the [`ConfigWarning`] vec.
pub fn resolve_paths(
    env: &dyn EnvAccess,
    cli: &CliOverrides,
) -> Result<(NushellConfigDirs, Vec<ConfigWarning>), ConfigError> {
    let mut warnings = Vec::new();

    // ── config_home ──────────────────────────────────────────────────────
    let (config_home, xdg_was_absolute) = resolve_config_home_with_source(env, cli)?;

    // ── config_file / env_file ───────────────────────────────────────────
    let config_file = config_path_from_cli(cli.config_file.as_ref(), config_home.join("config.nu"));
    let env_file = config_path_from_cli(cli.env_file.as_ref(), config_home.join("env.nu"));

    // ── data_home ────────────────────────────────────────────────────────
    let data_home = resolve_xdg_base(env, "XDG_DATA_HOME", |e| e.data_dir())
        .unwrap_or_else(|| config_home.join("data"))
        .join("nushell");

    // ── cache_home ───────────────────────────────────────────────────────
    let cache_home = resolve_xdg_base(env, "XDG_CACHE_HOME", |e| e.cache_dir())
        .unwrap_or_else(|| config_home.join("cache"))
        .join("nushell");

    // ── home_dir ─────────────────────────────────────────────────────────
    let home_dir = env.home_dir().ok_or(ConfigError::NoHomeDir)?;

    // ── vendor_autoload_dirs ─────────────────────────────────────────────
    let vendor_autoload_dirs = resolve_vendor_autoload_dirs(env);

    // ── user_autoload_dirs ───────────────────────────────────────────────
    let user_autoload_dirs = vec![config_home.join("autoload")];

    // ── plugin_file ──────────────────────────────────────────────────────
    #[cfg(feature = "plugin")]
    let plugin_file = config_path_from_cli(
        cli.plugin_file.as_ref(),
        config_home.join("plugin.msgpackz"),
    );

    // ── XDG validation (moved from main.rs) ──────────────────────────────
    // If the user set XDG_CONFIG_HOME but the platform default was used
    // (because the value was empty or non-absolute), emit a warning.
    if let Some(xdg_raw) = env.var("XDG_CONFIG_HOME") {
        if !xdg_raw.is_empty() && !xdg_was_absolute {
            warnings.push(ConfigWarning::XdgConfigIgnored {
                xdg: xdg_raw,
                resolved: config_home.clone(),
            });
        }

        // If XDG_CONFIG_HOME was used but the old dir still has files while
        // the new one is empty, warn about the migration.
        if xdg_was_absolute && let Some(old_config) = env.config_dir().map(|p| p.join("nushell")) {
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

/// Build a [`ConfigPath`] from an optional CLI path and a default path.
///
/// Centralizes the Default vs Override decision so resolve sites stay DRY.
fn config_path_from_cli(cli: Option<&PathBuf>, default: PathBuf) -> ConfigPath {
    match cli {
        Some(path) => ConfigPath::Override(path.clone()),
        None => ConfigPath::Default(default),
    }
}

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
    if let Some(xdg) = env.var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
        && Path::new(&xdg).is_absolute()
    {
        let mut home = PathBuf::from(xdg);
        home.push("nushell");
        return Ok((home, true));
    }

    // 3. Platform default.
    let base = env.config_dir().ok_or(ConfigError::ConfigDirNotFound)?;
    Ok((base.join("nushell"), false))
}

/// Resolve an XDG base directory (CONFIG, DATA, CACHE) with fallback.
fn resolve_xdg_base(
    env: &dyn EnvAccess,
    var_name: &str,
    fallback: impl FnOnce(&dyn EnvAccess) -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(val) = env.var(var_name)
        && !val.is_empty()
        && Path::new(&val).is_absolute()
    {
        return Some(PathBuf::from(val));
    }
    fallback(env)
}

/// Resolve vendor autoload directories.
///
/// Order is load order: earlier entries run first, later entries can override.
/// On Unix, `XDG_DATA_DIRS` entries are **reversed** so that earlier entries
/// in the env var have higher precedence (standard XDG behavior).
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
        use std::os::unix::ffi::OsStrExt;

        let data_dirs: OsString = env.var_os("XDG_DATA_DIRS").unwrap_or_else(|| {
            option_env!("PREFIX").map_or_else(
                || OsString::from("/usr/local/share/:/usr/share/"),
                |prefix| {
                    if prefix.ends_with("local") {
                        OsString::from(format!("{prefix}/share"))
                    } else {
                        OsString::from(format!("{prefix}/local/share:{prefix}/share"))
                    }
                },
            )
        });

        // Reverse so earlier XDG_DATA_DIRS entries load later and win.
        data_dirs
            .as_encoded_bytes()
            .split(|b| *b == b':')
            .map(|split| into_autoload_path(PathBuf::from(std::ffi::OsStr::from_bytes(split))))
            .rev()
            .for_each(&mut append);
    }

    #[cfg(windows)]
    {
        if let Some(path) = env.program_data_dir() {
            append(into_autoload_path(path));
        }
    }

    if let Some(path) = option_env!("NU_VENDOR_AUTOLOAD_DIR") {
        append(PathBuf::from(path));
    }

    if let Some(data_dir) = resolve_xdg_base(env, "XDG_DATA_HOME", |e| e.data_dir()) {
        append(into_autoload_path(data_dir));
    }

    if let Some(path) = env.var_os("NU_VENDOR_AUTOLOAD_DIR")
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
    use std::collections::HashMap;

    /// Helper: an absolute path for use in tests, on any platform.
    fn abs_path(components: &[&str]) -> PathBuf {
        let mut p = std::env::temp_dir();
        for c in components {
            p.push(c);
        }
        p
    }

    fn test_env_with_platform(vars: HashMap<String, String>) -> TestEnv {
        let config = abs_path(&["nu-config-test-config"]);
        let data = abs_path(&["nu-config-test-data"]);
        let cache = abs_path(&["nu-config-test-cache"]);
        let home = abs_path(&["nu-config-test-home"]);
        TestEnv::new(vars)
            .with_config_dir(config)
            .with_data_dir(data)
            .with_cache_dir(cache)
            .with_home_dir(home)
    }

    #[test]
    fn test_resolve_paths_uses_xdg_config_home() {
        let xdg_base = abs_path(&["xdg-test"]);
        let xdg_str = xdg_base.to_string_lossy().to_string();

        let mut vars = HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), xdg_str);
        let env = test_env_with_platform(vars);
        let (dirs, _warnings) =
            resolve_paths(&env, &CliOverrides::default()).expect("resolve should succeed");

        let expected_home = xdg_base.join("nushell");
        assert_eq!(dirs.config_home, expected_home);
        assert_eq!(
            dirs.config_file,
            ConfigPath::Default(expected_home.join("config.nu"))
        );
        assert_eq!(
            dirs.env_file,
            ConfigPath::Default(expected_home.join("env.nu"))
        );
        assert!(!dirs.config_file.is_override());
        assert!(!dirs.env_file.is_override());
    }

    #[test]
    fn test_resolve_paths_config_home_override() {
        let ignored = abs_path(&["xdg-ignored"]).to_string_lossy().to_string();
        let override_path = abs_path(&["my-override"]);

        let mut vars = HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), ignored);
        let env = test_env_with_platform(vars);
        let cli = CliOverrides {
            config_home: Some(override_path.clone()),
            ..Default::default()
        };
        let (dirs, _warnings) = resolve_paths(&env, &cli).expect("resolve should succeed");
        assert_eq!(dirs.config_home, override_path);
        assert_eq!(
            dirs.config_file,
            ConfigPath::Default(override_path.join("config.nu"))
        );
        assert_eq!(
            dirs.user_autoload_dirs,
            vec![override_path.join("autoload")]
        );
    }

    #[test]
    fn test_resolve_paths_cli_config_file_override() {
        let custom_root = abs_path(&["custom", "path"]);

        let env = test_env_with_platform(HashMap::new());
        let cli = CliOverrides {
            config_file: Some(custom_root.join("config.nu")),
            env_file: Some(custom_root.join("env.nu")),
            ..Default::default()
        };
        let (dirs, _warnings) = resolve_paths(&env, &cli).expect("resolve should succeed");
        assert_eq!(
            dirs.config_file,
            ConfigPath::Override(custom_root.join("config.nu"))
        );
        assert_eq!(
            dirs.env_file,
            ConfigPath::Override(custom_root.join("env.nu"))
        );
        assert!(dirs.config_file.is_override());
        assert!(dirs.env_file.is_override());
    }

    #[test]
    fn test_resolve_paths_empty_xdg_falls_back() {
        let mut vars = HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), "".into());
        let env = test_env_with_platform(vars);
        let (dirs, warnings) =
            resolve_paths(&env, &CliOverrides::default()).expect("empty XDG should fall back");
        assert!(dirs.config_home.ends_with("nushell"));
        // Empty string is not non-empty invalid, so no XdgConfigIgnored
        assert!(
            !warnings
                .iter()
                .any(|w| matches!(w, ConfigWarning::XdgConfigIgnored { .. }))
        );
    }

    #[test]
    fn test_resolve_paths_relative_xdg_ignored() {
        let mut vars = HashMap::new();
        vars.insert("XDG_CONFIG_HOME".into(), "relative/path".into());
        let env = test_env_with_platform(vars);
        let (dirs, warnings) =
            resolve_paths(&env, &CliOverrides::default()).expect("relative XDG should be ignored");
        assert!(dirs.config_home.ends_with("nushell"));
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, ConfigWarning::XdgConfigIgnored { .. }))
        );
    }

    #[test]
    fn test_resolve_paths_missing_config_dir_errors() {
        let env = TestEnv::new(HashMap::new()).with_home_dir(abs_path(&["home"]));
        // no config_dir set → ConfigDirNotFound
        let err = resolve_paths(&env, &CliOverrides::default()).unwrap_err();
        assert_eq!(err, ConfigError::ConfigDirNotFound);
    }

    #[test]
    fn test_resolve_paths_missing_home_errors() {
        let env = TestEnv::new(HashMap::new()).with_config_dir(abs_path(&["cfg"]));
        // no home_dir set → NoHomeDir
        let err = resolve_paths(&env, &CliOverrides::default()).unwrap_err();
        assert_eq!(err, ConfigError::NoHomeDir);
    }

    #[cfg(unix)]
    #[test]
    fn test_vendor_autoload_xdg_data_dirs_reversed() {
        let mut vars = HashMap::new();
        vars.insert("XDG_DATA_DIRS".into(), "/first/share:/second/share".into());
        let env = test_env_with_platform(vars);
        let (dirs, _) = resolve_paths(&env, &CliOverrides::default()).unwrap();

        let first = PathBuf::from("/first/share/nushell/vendor/autoload");
        let second = PathBuf::from("/second/share/nushell/vendor/autoload");

        let first_idx = dirs
            .vendor_autoload_dirs
            .iter()
            .position(|p| p == &first)
            .expect("first dir present");
        let second_idx = dirs
            .vendor_autoload_dirs
            .iter()
            .position(|p| p == &second)
            .expect("second dir present");

        // Reversed: second appears before first so first wins when loading.
        assert!(
            second_idx < first_idx,
            "expected reverse order, got: {:?}",
            dirs.vendor_autoload_dirs
        );
    }

    #[test]
    fn test_data_and_cache_use_xdg_when_absolute() {
        let xdg_data = abs_path(&["xdg-data"]);
        let xdg_cache = abs_path(&["xdg-cache"]);
        let mut vars = HashMap::new();
        vars.insert(
            "XDG_DATA_HOME".into(),
            xdg_data.to_string_lossy().into_owned(),
        );
        vars.insert(
            "XDG_CACHE_HOME".into(),
            xdg_cache.to_string_lossy().into_owned(),
        );
        let env = test_env_with_platform(vars);
        let (dirs, _) = resolve_paths(&env, &CliOverrides::default()).unwrap();
        assert_eq!(dirs.data_home, xdg_data.join("nushell"));
        assert_eq!(dirs.cache_home, xdg_cache.join("nushell"));
    }

    #[test]
    fn test_data_and_cache_fall_back_under_config_home() {
        let env = test_env_with_platform(HashMap::new());
        let (dirs, _) = resolve_paths(&env, &CliOverrides::default()).unwrap();
        assert!(dirs.data_home.ends_with("nushell"));
        assert!(dirs.cache_home.ends_with("nushell"));
        assert!(dirs.is_resolved());
    }

    #[test]
    fn test_user_autoload_is_under_config_home() {
        let env = test_env_with_platform(HashMap::new());
        let (dirs, _) = resolve_paths(&env, &CliOverrides::default()).unwrap();
        assert_eq!(
            dirs.user_autoload_dirs,
            vec![dirs.config_home.join("autoload")]
        );
    }

    #[test]
    fn test_config_path_from_cli_helper() {
        let default = PathBuf::from("/default/config.nu");
        assert_eq!(
            config_path_from_cli(None, default.clone()),
            ConfigPath::Default(default)
        );
        let custom = PathBuf::from("/custom.nu");
        assert_eq!(
            config_path_from_cli(Some(&custom), PathBuf::from("/unused")),
            ConfigPath::Override(custom)
        );
    }

    #[cfg(feature = "plugin")]
    #[test]
    fn test_plugin_file_default_and_override() {
        let env = test_env_with_platform(HashMap::new());
        let (dirs, _) = resolve_paths(&env, &CliOverrides::default()).unwrap();
        assert!(!dirs.plugin_file.is_override());
        assert_eq!(
            dirs.plugin_file.as_path(),
            dirs.config_home.join("plugin.msgpackz")
        );

        let custom = abs_path(&["plug", "registry.msgpackz"]);
        let cli = CliOverrides {
            plugin_file: Some(custom.clone()),
            ..Default::default()
        };
        let (dirs, _) = resolve_paths(&env, &cli).unwrap();
        assert_eq!(dirs.plugin_file, ConfigPath::Override(custom));
    }
}
