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

use nu_config::{CliOverrides, ConfigPath, NushellConfigDirs, SystemEnv, resolve_paths};
use nu_path::{AbsolutePath, AbsolutePathBuf, Path as NuPath};
use nu_protocol::{HistoryConfig, HistoryFileFormat, HistoryPath};
use nu_test_support::fs::executable_path;
use nu_test_support::playground::{Executable, Playground};
use nu_test_support::prelude::*;
use nu_test_support::tester::NuTester;
use pretty_assertions::assert_eq;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── helpers ──────────────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn adjust_canonicalization<P: AsRef<NuPath>>(p: P) -> String {
    p.as_ref().display().to_string()
}

#[cfg(target_os = "windows")]
fn adjust_canonicalization<P: AsRef<NuPath>>(p: P) -> String {
    const VERBATIM_PREFIX: &str = r"\\?\";
    let p = p.as_ref().display().to_string();
    if let Some(stripped) = p.strip_prefix(VERBATIM_PREFIX) {
        stripped.to_string()
    } else {
        p
    }
}

/// Default Nushell config directory, ignoring `XDG_CONFIG_HOME`.
fn non_xdg_config_dir() -> AbsolutePathBuf {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let config_dir = dirs::config_dir().expect("Could not get config directory");

    // On Linux, dirs::config_dir checks $XDG_CONFIG_HOME first, then $HOME/.config.
    #[cfg(target_os = "linux")]
    let config_dir = {
        let mut dir = dirs::home_dir().expect("Could not get config directory");
        dir.push(".config");
        dir
    };

    let config_dir = config_dir.canonicalize().unwrap_or(config_dir);
    let mut config_dir_nushell =
        AbsolutePathBuf::try_from(config_dir).expect("Invalid config directory");
    config_dir_nushell.push("nushell");
    if let Ok(canon) = config_dir_nushell.canonicalize() {
        canon.into_absolute()
    } else {
        config_dir_nushell
    }
}

/// Run a short `-c` script via the playground (real `nu` binary + env).
fn run_in_playground(playground: &mut Playground, command: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        playground.with_env("HOME", home.as_str());
    }
    let result = playground.pipeline(command).execute().map_err(|e| {
        let outcome = e.output.map(|outcome| {
            format!(
                "out: '{}', err: '{}'",
                String::from_utf8_lossy(&outcome.out),
                String::from_utf8_lossy(&outcome.err)
            )
        });
        format!(
            "desc: {}, exit: {:?}, outcome: {}",
            e.desc,
            e.exit,
            outcome.unwrap_or("empty".to_owned())
        )
    });
    String::from_utf8_lossy(&result.unwrap().out)
        .trim()
        .to_string()
}

/// Spawn the built `nu` binary with explicit args (replaces nested `nu!` usage).
fn run_nu(cwd: impl AsRef<std::path::Path>, args: &[&str]) -> (String, String) {
    let output = Command::new(executable_path())
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to spawn nu");
    (
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    )
}

/// Inject resolved dirs into a tester and regenerate `$nu`.
fn tester_with_dirs(dirs: NushellConfigDirs) -> NuTester {
    let mut tester = test();
    tester.engine_state.config_dirs = dirs;
    #[cfg(feature = "plugin")]
    {
        tester.engine_state.plugin_path =
            Some(tester.engine_state.config_dirs.plugin_file.to_path_buf());
    }
    tester.engine_state.generate_nu_constant();
    tester
}

fn abs_join(base: &Path, parts: &[&str]) -> PathBuf {
    let mut p = base.to_path_buf();
    for part in parts {
        p.push(part);
    }
    p
}

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
fn history_path_directory_appends_filename_plaintext() {
    // temp_dir exists as a directory, so the helper appends the default file name.
    let dir = std::env::temp_dir();
    let config = HistoryConfig {
        path: HistoryPath::Custom(dir.clone()),
        file_format: HistoryFileFormat::Plaintext,
        ..Default::default()
    };
    assert_eq!(
        config.file_path(std::path::Path::new("/unused")),
        Some(dir.join("history.txt"))
    );
}

#[test]
fn history_path_directory_appends_filename_sqlite() {
    let dir = std::env::temp_dir();
    let config = HistoryConfig {
        path: HistoryPath::Custom(dir.clone()),
        file_format: HistoryFileFormat::Sqlite,
        ..Default::default()
    };
    assert_eq!(
        config.file_path(std::path::Path::new("/unused")),
        Some(dir.join("history.sqlite3"))
    );
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
    let home = abs_join(&std::env::temp_dir(), &["nu-test-cfg-home"]);
    let _ = fs::create_dir_all(&home);

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

    let mut tester = tester_with_dirs(dirs);

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
}

#[test]
fn nu_constant_config_file_override() -> Result {
    let home = abs_join(&std::env::temp_dir(), &["nu-test-cfg-override-home"]);
    let custom = abs_join(&std::env::temp_dir(), &["nu-test-custom-config.nu"]);
    let _ = fs::create_dir_all(&home);
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

    let mut tester = tester_with_dirs(dirs);
    let config_path: String = tester.run("$nu.config-path")?;
    assert!(
        config_path.contains("nu-test-custom-config.nu"),
        "config-path={config_path}"
    );
    Ok(())
}

#[test]
fn nu_constant_history_follows_config_home() -> Result {
    let home = abs_join(&std::env::temp_dir(), &["nu-hist-config-home"]);
    let _ = fs::create_dir_all(&home);

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

    let mut tester = tester_with_dirs(dirs);
    let history_path: String = tester.run("$nu.history-path")?;
    assert!(
        history_path.contains("nu-hist-config-home") && history_path.contains("history"),
        "history-path={history_path}"
    );
    Ok(())
}

// ─── 3. Subprocess / playground (startup + CLI) ───────────────────────────

fn assert_config_paths_in_playground(
    playground: &mut Playground,
    config_dir_nushell: impl AsRef<AbsolutePath>,
) {
    let config_dir_nushell = config_dir_nushell.as_ref();

    if !config_dir_nushell.exists() {
        let _ = fs::create_dir_all(config_dir_nushell);
    }

    let config_dir_nushell =
        std::path::absolute(config_dir_nushell).expect("canonicalize config dir failed");
    let actual = run_in_playground(playground, "$nu.default-config-dir");
    assert_eq!(actual, adjust_canonicalization(&config_dir_nushell));

    let config_path = config_dir_nushell.join("config.nu");
    let canon_config_path =
        adjust_canonicalization(std::path::absolute(&config_path).unwrap_or(config_path));
    assert_eq!(
        run_in_playground(playground, "$nu.config-path"),
        canon_config_path
    );

    let env_path = config_dir_nushell.join("env.nu");
    let canon_env_path =
        adjust_canonicalization(std::path::absolute(&env_path).unwrap_or(env_path));
    assert_eq!(
        run_in_playground(playground, "$nu.env-path"),
        canon_env_path
    );

    let history_path = config_dir_nushell.join("history.txt");
    let canon_history_path =
        adjust_canonicalization(std::path::absolute(&history_path).unwrap_or(history_path));
    assert_eq!(
        run_in_playground(playground, "$nu.history-path"),
        canon_history_path
    );

    let login_path = config_dir_nushell.join("login.nu");
    let canon_login_path =
        adjust_canonicalization(std::path::absolute(&login_path).unwrap_or(login_path));
    assert_eq!(
        run_in_playground(playground, "$nu.loginshell-path"),
        canon_login_path
    );

    #[cfg(feature = "plugin")]
    {
        let plugin_path = config_dir_nushell.join("plugin.msgpackz");
        let canon_plugin_path =
            adjust_canonicalization(std::path::absolute(&plugin_path).unwrap_or(plugin_path));
        assert_eq!(
            run_in_playground(playground, "$nu.plugin-path"),
            canon_plugin_path
        );
    }
}

#[test]
fn test_default_config_path() {
    Playground::setup("default_config_path", |_, playground| {
        assert_config_paths_in_playground(playground, non_xdg_config_dir());
    });
}

#[test]
fn test_alternate_config_path() {
    let config_file = "crates/nu-config/src/default_files/scaffold_config.nu";
    let env_file = "crates/nu-config/src/default_files/scaffold_env.nu";
    let cwd = std::env::current_dir().expect("cwd");

    let config_path =
        nu_path::canonicalize_with(config_file, &cwd).expect("Could not get config path");
    let (out, _) = run_nu(
        &cwd,
        &[
            "--config",
            &config_path.to_string_lossy(),
            "-c",
            "$nu.config-path",
        ],
    );
    assert_eq!(out, config_path.to_string_lossy());

    let env_path = nu_path::canonicalize_with(env_file, &cwd).expect("Could not get env path");
    let (out, _) = run_nu(
        &cwd,
        &[
            "--env-config",
            &env_path.to_string_lossy(),
            "-c",
            "$nu.env-path",
        ],
    );
    assert_eq!(out, env_path.to_string_lossy());
}

#[test]
fn use_last_config_path() {
    let config_file = "crates/nu-config/src/default_files/scaffold_config.nu";
    let env_file = "crates/nu-config/src/default_files/scaffold_env.nu";
    let cwd = std::env::current_dir().expect("cwd");

    let config_path =
        nu_path::canonicalize_with(config_file, &cwd).expect("Could not get config path");
    let (out, _) = run_nu(
        &cwd,
        &[
            "--config",
            "non-existing-path",
            "--config",
            "another-random-path.nu",
            "--config",
            &config_path.to_string_lossy(),
            "-c",
            "$nu.config-path",
        ],
    );
    assert_eq!(out, config_path.to_string_lossy());

    let env_path = nu_path::canonicalize_with(env_file, &cwd).expect("Could not get env path");
    let (out, _) = run_nu(
        &cwd,
        &[
            "--env-config",
            "non-existing-path",
            "--env-config",
            &env_path.to_string_lossy(),
            "-c",
            "$nu.env-path",
        ],
    );
    assert_eq!(out, env_path.to_string_lossy());
}

#[test]
fn test_xdg_config_empty() {
    Playground::setup("xdg_config_empty", |_, playground| {
        playground.with_env("XDG_CONFIG_HOME", "");
        let actual = run_in_playground(playground, "$nu.default-config-dir");
        let expected = non_xdg_config_dir();
        assert_eq!(actual, adjust_canonicalization(expected));
    });
}

#[test]
fn test_xdg_config_bad() {
    Playground::setup("xdg_config_bad", |_, playground| {
        let xdg_config_home = r#"mn2''6t\/k*((*&^//k//: "#;
        playground.with_env("XDG_CONFIG_HOME", xdg_config_home);

        let actual = run_in_playground(playground, "$nu.default-config-dir");
        let expected = non_xdg_config_dir();
        assert_eq!(actual, adjust_canonicalization(expected));

        #[cfg(not(windows))]
        {
            let child = Command::new(executable_path())
                .arg("-i")
                .arg("-c")
                .arg("echo $nu.is-interactive")
                .env("XDG_CONFIG_HOME", adjust_canonicalization(xdg_config_home))
                .output()
                .expect("Should have outputted");
            let stderr = String::from_utf8_lossy(&child.stderr);
            assert!(
                stderr.contains("xdg_config_home_invalid"),
                "stderr was {stderr}"
            );
        }
    });
}

/// Shouldn't complain if XDG_CONFIG_HOME is a symlink.
#[test]
#[cfg(not(windows))]
fn test_xdg_config_symlink() {
    Playground::setup("xdg_config_symlink", |_, playground| {
        let config_link = "config_link";
        playground.symlink("real", config_link);

        let child = Command::new(executable_path())
            .arg("-i")
            .arg("-c")
            .arg("echo $nu.is-interactive")
            .env(
                "XDG_CONFIG_HOME",
                adjust_canonicalization(playground.cwd().join(config_link)),
            )
            .output()
            .expect("Should have outputted");
        let stderr = String::from_utf8_lossy(&child.stderr);
        assert!(
            !stderr.contains("xdg_config_home_invalid"),
            "stderr was {stderr}"
        );
    });
}

#[test]
fn no_config_does_not_load_env_files() {
    let (out, _) = run_nu(
        std::env::current_dir().unwrap(),
        &[
            "-n",
            "-c",
            "view files | where filename =~ 'env\\.nu$' | length",
        ],
    );
    assert_eq!(out, "0");
}

#[test]
fn no_config_does_not_load_config_files() {
    let (out, _) = run_nu(
        std::env::current_dir().unwrap(),
        &[
            "-n",
            "-c",
            "view files | where filename =~ 'config\\.nu$' | length",
        ],
    );
    assert_eq!(out, "0");
}

#[test]
fn commandstring_does_not_load_config_files() {
    let (out, _) = run_nu(
        std::env::current_dir().unwrap(),
        &[
            "-c",
            "view files | where filename =~ 'config\\.nu$' | length",
        ],
    );
    assert_eq!(out, "0");
}

#[test]
fn commandstring_does_not_load_user_env() {
    let (out, _) = run_nu(
        std::env::current_dir().unwrap(),
        &[
            "-c",
            "view files | where filename =~ '[^_]env\\.nu$' | length",
        ],
    );
    assert_eq!(out, "0");
}

#[test]
fn commandstring_loads_default_env() {
    let (out, _) = run_nu(
        std::env::current_dir().unwrap(),
        &[
            "-c",
            "view files | where filename =~ 'default_env\\.nu$' | length",
        ],
    );
    assert_eq!(out, "1");
}

#[test]
fn commandstring_populates_config_record() {
    let (out, _) = run_nu(
        std::env::current_dir().unwrap(),
        &["--no-std-lib", "-n", "-c", "$env.config.show_banner"],
    );
    assert_eq!(out, "true");
}

#[test]
fn history_path_disabled_null() {
    Playground::setup("history_null", |_, playground| {
        let config_path = playground.cwd().join("config.nu");
        std::fs::write(&config_path, "$env.config.history.path = null").unwrap();

        let (out, _) = run_nu(
            playground.cwd(),
            &[
                "--config",
                &config_path.to_string_lossy(),
                "-c",
                "$nu.history-path",
            ],
        );
        assert_eq!(out, "");
    });
}

#[test]
fn history_path_custom_string() {
    Playground::setup("history_custom", |_, playground| {
        let custom_file = playground.cwd().join("my_history.txt");
        let config_path = playground.cwd().join("config.nu");
        std::fs::write(
            &config_path,
            format!(
                "$env.config.history.path = '{}'",
                custom_file.to_string_lossy()
            ),
        )
        .unwrap();

        let (out, _) = run_nu(
            playground.cwd(),
            &[
                "--config",
                &config_path.to_string_lossy(),
                "-c",
                "$nu.history-path",
            ],
        );
        assert_eq!(out, custom_file.to_string_lossy());
    });
}

#[test]
fn history_path_default_shows_in_config() {
    let (out, _) = run_nu(
        std::env::current_dir().unwrap(),
        &["--no-std-lib", "-n", "-c", "$env.config.history.path"],
    );
    assert_eq!(out, "");
}

#[test]
fn config_home_cli_affects_default_config_dir() {
    Playground::setup("config_home_cli", |_, playground| {
        let alt_home = playground.cwd().join("alt-config-home");
        std::fs::create_dir_all(&alt_home).unwrap();

        let (out, _) = run_nu(
            playground.cwd(),
            &[
                "--no-std-lib",
                "-n",
                "--config-home",
                &alt_home.to_string_lossy(),
                "-c",
                "$nu.default-config-dir",
            ],
        );

        let expected = match alt_home.canonicalize() {
            Ok(canon) => adjust_canonicalization(canon),
            Err(_) => adjust_canonicalization(&alt_home),
        };
        assert_eq!(out, expected);
    });
}

#[test]
fn config_home_cli_affects_history_path() {
    Playground::setup("config_home_history", |_, playground| {
        let alt_home = playground.cwd().join("alt-hist-home");
        std::fs::create_dir_all(&alt_home).unwrap();

        let (out, _) = run_nu(
            playground.cwd(),
            &[
                "--no-std-lib",
                "-n",
                "--config-home",
                &alt_home.to_string_lossy(),
                "-c",
                "$nu.history-path",
            ],
        );

        assert!(
            out.contains("alt-hist-home") && out.contains("history"),
            "history-path should live under --config-home, got: {out}"
        );
    });
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
