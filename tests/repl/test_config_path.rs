//! Config path resolution and `$nu.*` path fields.
//!
//! # Layers under test
//!
//! 1. **Pure history helpers** — [`HistoryConfig::file_path`] with an explicit
//!    `config_home` (no process spawn).
//! 2. **In-process engine** — [`NuTester`] with injected [`NushellConfigDirs`]
//!    to verify `$nu` reflects resolved dirs (no `nu` binary).
//! 3. **CLI / env subprocess** — real `nu` binary via playground / process when
//!    startup path resolution or CLI flags must be exercised.
//!
//! Prefer (1) and (2). Use (3) only for end-to-end startup behavior.

use nu_config::{
    CliOverrides, ConfigPath, ConfigWarning, NushellConfigDirs, SystemEnv, resolve_paths,
};
use nu_protocol::{Config, HistoryConfig, HistoryFileFormat, HistoryPath};
use nu_test_support::prelude::*;
use pretty_assertions::assert_eq;
use std::fs;
use std::path::PathBuf;
#[cfg(not(windows))]
use std::process::Command;

// ─── 1. Pure HistoryConfig unit tests ─────────────────────────────────────

#[test]
fn history_config_disabled() {
    let config = HistoryConfig {
        path: HistoryPath::Disabled,
        ..Default::default()
    };
    assert_eq!(config.file_path(std::path::Path::new("/tmp")), None);
}

#[test]
fn history_config_default_path_plaintext() {
    let config_dir = PathBuf::from("/resolved/config-home");
    let config = HistoryConfig {
        path: HistoryPath::Default,
        file_format: HistoryFileFormat::Plaintext,
        ..Default::default()
    };
    assert_eq!(
        config.file_path(&config_dir),
        Some(config_dir.join("history.txt"))
    );
}

#[test]
fn history_config_default_path_sqlite() {
    let config_dir = PathBuf::from("/resolved/config-home");
    let config = HistoryConfig {
        path: HistoryPath::Default,
        file_format: HistoryFileFormat::Sqlite,
        ..Default::default()
    };
    assert_eq!(
        config.file_path(&config_dir),
        Some(config_dir.join("history.sqlite3"))
    );
}

#[test]
fn history_path_directory_appends_filename_plaintext() -> Result {
    Playground::setup("history_path_dir_plaintext", |dirs, _| {
        let dir = dirs.test().to_std_path_buf();
        let config = HistoryConfig {
            path: HistoryPath::Custom(dir.clone()),
            file_format: HistoryFileFormat::Plaintext,
            ..Default::default()
        };
        assert_eq!(
            config.file_path(std::path::Path::new("/unused")),
            Some(dir.join("history.txt"))
        );
        Ok(())
    })
}

#[test]
fn history_path_directory_appends_filename_sqlite() -> Result {
    Playground::setup("history_path_dir_sqlite", |dirs, _| {
        let dir = dirs.test().to_std_path_buf();
        let config = HistoryConfig {
            path: HistoryPath::Custom(dir.clone()),
            file_format: HistoryFileFormat::Sqlite,
            ..Default::default()
        };
        assert_eq!(
            config.file_path(std::path::Path::new("/unused")),
            Some(dir.join("history.sqlite3"))
        );
        Ok(())
    })
}

#[test]
fn history_path_empty_string_means_default() {
    let config = HistoryConfig {
        path: HistoryPath::Default,
        ..Default::default()
    };
    let config_dir = PathBuf::from("/cfg");
    assert_eq!(
        config.file_path(&config_dir),
        Some(config_dir.join("history.txt"))
    );
}

/// `--config-home` must be honored for default history location (no env re-resolve).
#[test]
fn history_uses_injected_config_home_not_env() {
    let alt = PathBuf::from("/alt/config-home-only");
    let config = HistoryConfig {
        path: HistoryPath::Default,
        file_format: HistoryFileFormat::Plaintext,
        ..Default::default()
    };
    assert_eq!(
        config.file_path(&alt),
        Some(alt.join("history.txt")),
        "history must use the session config_home, not re-read XDG/env"
    );
}

// ─── 2. In-process $nu path tests (NuTester) ──────────────────────────────

#[test]
fn nu_constant_reflects_resolved_config_dirs() -> Result {
    Playground::setup("nu_constant_reflects_resolved_config_dirs", |dirs, _| {
        let home = dirs.test().join("nu-test-cfg-home").to_std_path_buf();
        fs::create_dir_all(&home)?;

        let dirs = NushellConfigDirs {
            config_home: home.clone(),
            config_file: ConfigPath::Default(home.join("config.nu")),
            env_file: ConfigPath::Default(home.join("env.nu")),
            data_home: home.join("data"),
            cache_home: home.join("cache"),
            home_dir: home.clone(),
            vendor_autoload_dirs: vec![home.join("vendor")],
            user_autoload_dirs: vec![home.join("autoload")],
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::Default(home.join("plugin.msgpackz")),
        };

        let mut tester = test();
        tester.engine_state.config_dirs = dirs;
        #[cfg(feature = "plugin")]
        {
            tester.engine_state.plugin_path =
                Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
        }
        tester.engine_state.generate_nu_constant();

        let default_dir: String = tester.run("$nu.default-config-dir")?;
        assert!(
            default_dir.contains("nu-test-cfg-home") || default_dir.ends_with("nu-test-cfg-home"),
            "default-config-dir={default_dir}"
        );

        let config_path: String = tester.run("$nu.config-path")?;
        assert!(
            config_path.ends_with("config.nu") || config_path.contains("config.nu"),
            "config-path={config_path}"
        );

        let env_path: String = tester.run("$nu.env-path")?;
        assert!(
            env_path.ends_with("env.nu") || env_path.contains("env.nu"),
            "env-path={env_path}"
        );

        let history_path: String = tester.run("$nu.history-path")?;
        assert!(
            history_path.contains("history"),
            "history-path={history_path}"
        );

        Ok(())
    })
}

#[test]
fn nu_constant_config_file_override() -> Result {
    Playground::setup("nu_constant_config_file_override", |dirs, _| {
        let home = dirs
            .test()
            .join("nu-test-cfg-override-home")
            .to_std_path_buf();
        let custom = dirs
            .test()
            .join("nu-test-custom-config.nu")
            .to_std_path_buf();
        fs::create_dir_all(&home)?;
        // File need not exist for $nu path reporting (canonicalize falls back).

        let dirs = NushellConfigDirs {
            config_home: home.clone(),
            config_file: ConfigPath::Override(custom.clone()),
            env_file: ConfigPath::Default(home.join("env.nu")),
            data_home: home.join("data"),
            cache_home: home.join("cache"),
            home_dir: home,
            vendor_autoload_dirs: vec![],
            user_autoload_dirs: vec![],
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::Default(PathBuf::from("plugin.msgpackz")),
        };

        let mut tester = test();
        tester.engine_state.config_dirs = dirs;
        #[cfg(feature = "plugin")]
        {
            tester.engine_state.plugin_path =
                Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
        }
        tester.engine_state.generate_nu_constant();
        let config_path: String = tester.run("$nu.config-path")?;
        assert!(
            config_path.contains("nu-test-custom-config.nu"),
            "config-path={config_path}"
        );
        Ok(())
    })
}

#[test]
fn nu_constant_history_follows_config_home() -> Result {
    Playground::setup("nu_constant_history_follows_config_home", |dirs, _| {
        let home = dirs.test().join("nu-hist-config-home").to_std_path_buf();
        fs::create_dir_all(&home)?;

        let dirs = NushellConfigDirs {
            config_home: home.clone(),
            config_file: ConfigPath::Default(home.join("config.nu")),
            env_file: ConfigPath::Default(home.join("env.nu")),
            data_home: home.join("data"),
            cache_home: home.join("cache"),
            home_dir: home.clone(),
            vendor_autoload_dirs: vec![],
            user_autoload_dirs: vec![],
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::Default(home.join("plugin.msgpackz")),
        };

        let mut tester = test();
        tester.engine_state.config_dirs = dirs;
        #[cfg(feature = "plugin")]
        {
            tester.engine_state.plugin_path =
                Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
        }
        tester.engine_state.generate_nu_constant();
        let history_path: String = tester.run("$nu.history-path")?;
        assert!(
            history_path.contains("nu-hist-config-home") && history_path.contains("history"),
            "history-path={history_path}"
        );
        Ok(())
    })
}

// ─── 3. Subprocess / playground (startup + CLI) ───────────────────────────

#[test]
#[env(XDG_CONFIG_HOME = "")]
fn test_default_config_path() -> Result {
    let (dirs, _) = resolve_paths(&SystemEnv, &CliOverrides::default()).unwrap();
    let mut tester = test();
    tester.engine_state.config_dirs = dirs.clone();
    #[cfg(feature = "plugin")]
    {
        tester.engine_state.plugin_path = tester
            .engine_state
            .config_dirs
            .plugin_file
            .to_path_buf()
            .canonicalize()
            .ok();
    }
    tester.engine_state.generate_nu_constant();

    let try_canonicalized = |p: PathBuf| {
        p.canonicalize()
            .ok() // windows!
            .map(|c| c.strip_prefix(r"\\?\").map(From::from).unwrap_or(c))
            .unwrap_or(p)
    };

    let home = dirs.config_home.to_path_buf();
    tester
        .run("$nu.default-config-dir")
        .expect_value_eq(try_canonicalized(home.clone()))?;

    let config = dirs.config_file.to_path_buf();
    tester
        .run("$nu.config-path")
        .expect_value_eq(try_canonicalized(config))?;

    let environment = dirs.env_file.to_path_buf();
    tester
        .run("$nu.env-path")
        .expect_value_eq(try_canonicalized(environment))?;

    let history = home.join("history.txt");
    tester
        .run("$nu.history-path")
        .expect_value_eq(try_canonicalized(history))?;

    let login = home.join("login.nu");
    tester
        .run("$nu.loginshell-path")
        .expect_value_eq(try_canonicalized(login))?;

    #[cfg(feature = "plugin")]
    {
        let plugin = dirs.plugin_file.to_path_buf();
        tester
            .run("$nu.plugin-path")
            .expect_value_eq(try_canonicalized(plugin))?;
    }

    Ok(())
}

#[test]
fn test_alternate_config_path() -> Result {
    let config_file = "crates/nu-config/default_files/scaffold_config.nu";
    let env_file = "crates/nu-config/default_files/scaffold_env.nu";
    let cwd = WORKSPACE_ROOT.as_path();

    let config_path = nu_path::canonicalize_with(config_file, cwd)?;
    let env_path = nu_path::canonicalize_with(env_file, cwd)?;

    let dirs = NushellConfigDirs {
        config_home: cwd.join("test-home"),
        config_file: ConfigPath::Override(config_path.clone()),
        env_file: ConfigPath::Override(env_path.clone()),
        data_home: cwd.join("data"),
        cache_home: cwd.join("cache"),
        home_dir: cwd.to_path_buf(),
        vendor_autoload_dirs: vec![],
        user_autoload_dirs: vec![],
        #[cfg(feature = "plugin")]
        plugin_file: ConfigPath::Default(cwd.join("plugin.msgpackz")),
    };

    let mut tester = test();
    tester.engine_state.config_dirs = dirs;
    #[cfg(feature = "plugin")]
    {
        tester.engine_state.plugin_path =
            Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
    }
    tester.engine_state.generate_nu_constant();
    tester.run("$nu.config-path").expect_value_eq(config_path)?;

    tester.run("$nu.env-path").expect_value_eq(env_path)?;

    Ok(())
}

#[test]
fn use_last_config_path() -> Result {
    let config_file = "crates/nu-config/default_files/scaffold_config.nu";
    let env_file = "crates/nu-config/default_files/scaffold_env.nu";
    let cwd = WORKSPACE_ROOT.as_path();

    let config_path = nu_path::canonicalize_with(config_file, cwd)?;
    let env_path = nu_path::canonicalize_with(env_file, cwd)?;

    let dirs = NushellConfigDirs {
        config_home: cwd.join("test-home"),
        config_file: ConfigPath::Override(config_path.clone()),
        env_file: ConfigPath::Override(env_path.clone()),
        data_home: cwd.join("data"),
        cache_home: cwd.join("cache"),
        home_dir: cwd.to_path_buf(),
        vendor_autoload_dirs: vec![],
        user_autoload_dirs: vec![],
        #[cfg(feature = "plugin")]
        plugin_file: ConfigPath::Default(cwd.join("plugin.msgpackz")),
    };

    let mut tester = test();
    tester.engine_state.config_dirs = dirs;
    #[cfg(feature = "plugin")]
    {
        tester.engine_state.plugin_path =
            Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
    }
    tester.engine_state.generate_nu_constant();
    tester.run("$nu.config-path").expect_value_eq(config_path)?;

    tester.run("$nu.env-path").expect_value_eq(env_path)?;

    Ok(())
}

#[test]
#[env(XDG_CONFIG_HOME = "")]
fn test_xdg_config_empty() -> Result {
    let (dirs, warnings) = resolve_paths(&SystemEnv, &CliOverrides::default()).unwrap();
    assert!(warnings.is_empty(), "warnings={warnings:?}");

    let mut tester = test();
    tester.engine_state.config_dirs = dirs.clone();
    tester.engine_state.generate_nu_constant();

    tester
        .run("$nu.default-config-dir")
        .expect_value_eq(dirs.config_home)?;
    Ok(())
}

#[test]
#[env(XDG_CONFIG_HOME = "mn2''6t\\/k*((*&^//k//: ")]
#[deps(NU)]
fn test_xdg_config_bad() -> Result {
    let xdg_config_home = r#"mn2''6t\/k*((*&^//k//: "#;
    let (dirs, warnings) = resolve_paths(&SystemEnv, &CliOverrides::default()).unwrap();
    assert!(
        matches!(
            warnings.as_slice(),
            [ConfigWarning::XdgConfigIgnored { xdg, .. }] if xdg == xdg_config_home
        ),
        "warnings={warnings:?}"
    );

    let mut tester = test();
    tester.engine_state.config_dirs = dirs.clone();
    tester.engine_state.generate_nu_constant();
    tester
        .run("$nu.default-config-dir")
        .expect_value_eq(dirs.config_home)?;

    #[cfg(not(windows))]
    {
        let result: CompleteResult = test()
            .env("XDG_CONFIG_HOME", xdg_config_home)
            .inherit_env_if_set("HOME")
            .run("nu -i -c 'echo $nu.is-interactive' | complete")?;
        assert_contains("xdg_config_home_invalid", result.stderr);
    }

    Ok(())
}

/// Shouldn't complain if XDG_CONFIG_HOME is a symlink.
#[test]
#[cfg(not(windows))]
#[deps(NU)]
fn test_xdg_config_symlink() -> Result {
    Playground::setup("xdg_config_symlink", |_, playground| {
        let config_link = "config_link";
        playground.symlink("real", config_link);

        let result: CompleteResult = test()
            .env(
                "XDG_CONFIG_HOME",
                playground.cwd().join(config_link).display().to_string(),
            )
            .inherit_env_if_set("HOME")
            .run("nu -i -c 'echo $nu.is-interactive' | complete")?;
        assert_contains_not("xdg_config_home_invalid", result.stderr);
        Ok(())
    })
}

#[test]
#[deps(NU)]
fn no_config_does_not_load_env_files() -> Result {
    let result: CompleteResult = test()
        .run(r#"nu -n -c "view files | where filename =~ 'env\\.nu$' | length" | complete"#)?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "0");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn no_config_does_not_load_config_files() -> Result {
    let result: CompleteResult = test()
        .run(r#"nu -n -c "view files | where filename =~ 'config\\.nu$' | length" | complete"#)?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "0");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn commandstring_does_not_load_config_files() -> Result {
    let result: CompleteResult = test()
        .run(r#"nu -c "view files | where filename =~ 'config\\.nu$' | length" | complete"#)?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "0");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn commandstring_does_not_load_user_env() -> Result {
    let result: CompleteResult = test()
        .run(r#"nu -c "view files | where filename =~ '[^_]env\\.nu$' | length" | complete"#)?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "0");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn commandstring_loads_default_env() -> Result {
    let result: CompleteResult = test()
        .run(r#"nu -c "view files | where filename =~ 'default_env\\.nu$' | length" | complete"#)?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "1");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
#[deps(NU)]
fn commandstring_populates_config_record() -> Result {
    let result: CompleteResult =
        test().run("nu --no-std-lib -n -c '$env.config.show_banner' | complete")?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "true");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
fn history_path_disabled_null() -> Result {
    Playground::setup("history_path_disabled_null", |dirs, _| {
        let home = dirs.test().join("nu-history-null-home").to_std_path_buf();
        fs::create_dir_all(&home)?;

        let dirs = NushellConfigDirs {
            config_home: home.clone(),
            config_file: ConfigPath::Default(home.join("config.nu")),
            env_file: ConfigPath::Default(home.join("env.nu")),
            data_home: home.join("data"),
            cache_home: home.join("cache"),
            home_dir: home.clone(),
            vendor_autoload_dirs: vec![],
            user_autoload_dirs: vec![],
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::Default(home.join("plugin.msgpackz")),
        };

        let mut tester = test();
        tester.engine_state.config_dirs = dirs;
        #[cfg(feature = "plugin")]
        {
            tester.engine_state.plugin_path =
                Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
        }
        tester.engine_state.generate_nu_constant();
        tester.engine_state.set_config(Config {
            history: HistoryConfig {
                path: HistoryPath::Disabled,
                ..Default::default()
            },
            ..Default::default()
        });
        tester.engine_state.generate_nu_constant();

        tester.run("$nu.history-path").expect_value_eq("")?;

        Ok(())
    })
}

#[test]
fn history_path_custom_string() -> Result {
    Playground::setup("history_path_custom_string", |dirs, _| {
        let home = dirs.test().join("nu-history-custom-home").to_std_path_buf();
        fs::create_dir_all(&home)?;
        let custom_file = home.join("my_history.txt");

        let dirs = NushellConfigDirs {
            config_home: home.clone(),
            config_file: ConfigPath::Default(home.join("config.nu")),
            env_file: ConfigPath::Default(home.join("env.nu")),
            data_home: home.join("data"),
            cache_home: home.join("cache"),
            home_dir: home.clone(),
            vendor_autoload_dirs: vec![],
            user_autoload_dirs: vec![],
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::Default(home.join("plugin.msgpackz")),
        };

        let mut tester = test();
        tester.engine_state.config_dirs = dirs;
        #[cfg(feature = "plugin")]
        {
            tester.engine_state.plugin_path =
                Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
        }
        tester.engine_state.generate_nu_constant();
        tester.engine_state.set_config(Config {
            history: HistoryConfig {
                path: HistoryPath::Custom(custom_file.clone()),
                ..Default::default()
            },
            ..Default::default()
        });
        tester.engine_state.generate_nu_constant();

        tester
            .run("$nu.history-path")
            .expect_value_eq(custom_file)?;

        Ok(())
    })
}

#[test]
#[deps(NU)]
fn history_path_default_shows_in_config() -> Result {
    let result: CompleteResult =
        test().run("nu --no-std-lib -n -c '$env.config.history.path' | complete")?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "");
    assert_eq!(result.stderr, "");
    Ok(())
}

#[test]
fn config_home_cli_affects_default_config_dir() -> Result {
    Playground::setup("config_home_cli", |_, playground| {
        let alt_home = playground.cwd().join("alt-config-home");
        std::fs::create_dir_all(&alt_home)?;

        let dirs = NushellConfigDirs {
            config_home: alt_home.clone().into(),
            config_file: ConfigPath::Default(alt_home.join("config.nu").into()),
            env_file: ConfigPath::Default(alt_home.join("env.nu").into()),
            data_home: alt_home.join("data").into(),
            cache_home: alt_home.join("cache").into(),
            home_dir: alt_home.clone().into(),
            vendor_autoload_dirs: vec![],
            user_autoload_dirs: vec![],
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::Default(alt_home.join("plugin.msgpackz").into()),
        };

        let mut tester = test();
        tester.engine_state.config_dirs = dirs;
        #[cfg(feature = "plugin")]
        {
            tester.engine_state.plugin_path =
                Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
        }
        tester.engine_state.generate_nu_constant();
        let expected = std::path::absolute(&alt_home)?;
        tester
            .run("$nu.default-config-dir")
            .expect_value_eq(expected)?;

        Ok(())
    })
}

#[test]
fn config_home_cli_affects_history_path() -> Result {
    Playground::setup("config_home_history", |_, playground| {
        let alt_home = playground.cwd().join("alt-hist-home");
        std::fs::create_dir_all(&alt_home)?;

        let dirs = NushellConfigDirs {
            config_home: alt_home.clone().into(),
            config_file: ConfigPath::Default(alt_home.join("config.nu").into()),
            env_file: ConfigPath::Default(alt_home.join("env.nu").into()),
            data_home: alt_home.join("data").into(),
            cache_home: alt_home.join("cache").into(),
            home_dir: alt_home.clone().into(),
            vendor_autoload_dirs: vec![],
            user_autoload_dirs: vec![],
            #[cfg(feature = "plugin")]
            plugin_file: ConfigPath::Default(alt_home.join("plugin.msgpackz").into()),
        };

        let mut tester = test();
        tester.engine_state.config_dirs = dirs;
        #[cfg(feature = "plugin")]
        {
            tester.engine_state.plugin_path =
                Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
        }
        tester.engine_state.generate_nu_constant();
        let history_path: String = tester.run("$nu.history-path")?;

        assert!(
            history_path.contains("alt-hist-home") && history_path.contains("history"),
            "history-path should live under config_home, got: {history_path}"
        );

        Ok(())
    })
}

/// Smoke: `resolve_paths` via system env still succeeds (no panic / empty crash).
#[test]
fn system_resolve_paths_smoke() {
    let result = resolve_paths(&SystemEnv, &CliOverrides::default());
    // On normal developer machines this succeeds; if it fails, error must be typed.
    match result {
        Ok((dirs, _)) => assert!(dirs.is_resolved() || dirs.config_home.as_os_str().is_empty()),
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("config directory") || msg.contains("home directory"),
                "unexpected error: {msg}"
            );
        }
    }
}

// ─── Broken / dangling config symlinks (#18710) ───────────────────────────

/// Default `config.nu` as a dangling symlink must not block startup: use
/// built-in defaults, warn on stderr, and never remove the symlink.
#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn dangling_config_nu_uses_defaults_with_warning() -> Result {
    Playground::setup("dangling_config_nu", |_, playground| {
        let xdg = playground.cwd().join("xdg");
        let nu_dir = xdg.join("nushell");
        fs::create_dir_all(&nu_dir)?;
        playground.symlink("/nonexistent/moved-repo/config.nu", "xdg/nushell/config.nu");

        let output = Command::new(NU.path())
            .current_dir(playground.cwd())
            .args(["-l", "-c", "print ok"])
            .env("XDG_CONFIG_HOME", &xdg)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "nu should start with dangling config.nu; stderr={stderr}"
        );
        assert!(
            stdout.contains("ok"),
            "expected command output, stdout={stdout}"
        );
        assert!(
            stderr.contains("broken symlink"),
            "expected broken-symlink warning, stderr={stderr}"
        );
        // Never delete or replace the user's symlink.
        assert!(
            nu_dir.join("config.nu").is_symlink(),
            "dangling config.nu symlink must be left in place"
        );
        Ok(())
    })
}

/// Default `env.nu` as a dangling symlink: same recovery as config.nu.
#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn dangling_env_nu_uses_defaults_with_warning() -> Result {
    Playground::setup("dangling_env_nu", |_, playground| {
        let xdg = playground.cwd().join("xdg");
        let nu_dir = xdg.join("nushell");
        fs::create_dir_all(&nu_dir)?;
        playground.symlink("/nonexistent/moved-repo/env.nu", "xdg/nushell/env.nu");

        let output = Command::new(NU.path())
            .current_dir(playground.cwd())
            .args(["-l", "-c", "print ok"])
            .env("XDG_CONFIG_HOME", &xdg)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "nu should start with dangling env.nu; stderr={stderr}"
        );
        assert!(stdout.contains("ok"), "stdout={stdout}");
        assert!(
            stderr.contains("broken symlink"),
            "expected broken-symlink warning, stderr={stderr}"
        );
        assert!(
            nu_dir.join("env.nu").is_symlink(),
            "dangling env.nu symlink must be left in place"
        );
        Ok(())
    })
}

/// Explicit `--config` to a dangling path remains a hard error in strict mode.
#[cfg(not(windows))]
#[test]
#[deps(NU)]
fn dangling_config_cli_override_still_errors() -> Result {
    Playground::setup("dangling_config_cli", |_, playground| {
        playground.symlink("/nonexistent/custom-config.nu", "broken-config.nu");
        let broken = playground.cwd().join("broken-config.nu");

        let output = Command::new(NU.path())
            .current_dir(playground.cwd())
            .arg("--config")
            .arg(&broken)
            .args(["-c", "print ok"])
            .output()?;

        assert!(
            !output.status.success(),
            "CLI --config to a missing/dangling path should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("File not found") || stderr.contains("not found"),
            "stderr={stderr}"
        );
        assert!(
            broken.is_symlink(),
            "CLI override must not remove the broken symlink"
        );
        Ok(())
    })
}
