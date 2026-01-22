use nu_path::{AbsolutePath, AbsolutePathBuf, Path};
use nu_test_support::nu;
use nu_test_support::playground::{Executable, Playground};
use pretty_assertions::assert_eq;
use std::fs;

#[cfg(not(target_os = "windows"))]
fn adjust_canonicalization<P: AsRef<Path>>(p: P) -> String {
    p.as_ref().display().to_string()
}

#[cfg(target_os = "windows")]
fn adjust_canonicalization<P: AsRef<Path>>(p: P) -> String {
    const VERBATIM_PREFIX: &str = r"\\?\";
    let p = p.as_ref().display().to_string();
    if let Some(stripped) = p.strip_prefix(VERBATIM_PREFIX) {
        stripped.to_string()
    } else {
        p
    }
}

/// The default Nushell config directory, ignoring XDG_CONFIG_HOME
fn non_xdg_config_dir() -> AbsolutePathBuf {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let config_dir = dirs::config_dir().expect("Could not get config directory");

    // On Linux, dirs::config_dir checks $XDG_CONFIG_HOME first, then gets $HOME/.config,
    // so we have to get $HOME ourselves
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
    config_dir_nushell
}

fn run(playground: &mut Playground, command: &str) -> String {
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

#[cfg(not(windows))]
fn run_interactive_stderr(xdg_config_home: impl AsRef<Path>) -> String {
    let child_output = std::process::Command::new(nu_test_support::fs::executable_path())
        .arg("-i")
        .arg("-c")
        .arg("echo $nu.is-interactive")
        .env("XDG_CONFIG_HOME", adjust_canonicalization(xdg_config_home))
        .output()
        .expect("Should have outputted");

    String::from_utf8_lossy(&child_output.stderr)
        .trim()
        .to_string()
}

fn test_config_path_helper(
    playground: &mut Playground,
    config_dir_nushell: impl AsRef<AbsolutePath>,
) {
    let config_dir_nushell = config_dir_nushell.as_ref();

    // Create the config dir folder structure if it does not already exist
    if !config_dir_nushell.exists() {
        let _ = fs::create_dir_all(config_dir_nushell);
    }

    let config_dir_nushell =
        std::path::absolute(config_dir_nushell).expect("canonicalize config dir failed");
    let actual = run(playground, "$nu.default-config-dir");
    assert_eq!(actual, adjust_canonicalization(&config_dir_nushell));

    let config_path = config_dir_nushell.join("config.nu");
    // We use canonicalize here in case the config or env is symlinked since $nu.config-path is returning the canonicalized path in #8653
    let canon_config_path =
        adjust_canonicalization(std::path::absolute(&config_path).unwrap_or(config_path));
    let actual = run(playground, "$nu.config-path");
    assert_eq!(actual, canon_config_path);

    let env_path = config_dir_nushell.join("env.nu");
    let canon_env_path =
        adjust_canonicalization(std::path::absolute(&env_path).unwrap_or(env_path));
    let actual = run(playground, "$nu.env-path");
    assert_eq!(actual, canon_env_path);

    let history_path = config_dir_nushell.join("history.txt");
    let canon_history_path =
        adjust_canonicalization(std::path::absolute(&history_path).unwrap_or(history_path));
    let actual = run(playground, "$nu.history-path");
    assert_eq!(actual, canon_history_path);

    let login_path = config_dir_nushell.join("login.nu");
    let canon_login_path =
        adjust_canonicalization(std::path::absolute(&login_path).unwrap_or(login_path));
    let actual = run(playground, "$nu.loginshell-path");
    assert_eq!(actual, canon_login_path);

    #[cfg(feature = "plugin")]
    {
        let plugin_path = config_dir_nushell.join("plugin.msgpackz");
        let canon_plugin_path =
            adjust_canonicalization(std::path::absolute(&plugin_path).unwrap_or(plugin_path));
        let actual = run(playground, "$nu.plugin-path");
        assert_eq!(actual, canon_plugin_path);
    }
}

/// Test that the config files are in the right places when XDG_CONFIG_HOME isn't set
#[test]
fn test_default_config_path() {
    Playground::setup("default_config_path", |_, playground| {
        test_config_path_helper(playground, non_xdg_config_dir());
    });
}

#[test]
fn test_alternate_config_path() {
    let config_file = "crates/nu-utils/src/default_files/scaffold_config.nu";
    let env_file = "crates/nu-utils/src/default_files/scaffold_env.nu";

    let cwd = std::env::current_dir().expect("Could not get current working directory");

    let config_path =
        nu_path::canonicalize_with(config_file, &cwd).expect("Could not get config path");
    let config_path_str = config_path.to_string_lossy();
    let actual = nu!(
        cwd: &cwd,
        format!("nu --config '{}' -c '$nu.config-path'", config_path_str)
    );
    assert_eq!(actual.out, config_path.to_string_lossy().to_string());

    let env_path = nu_path::canonicalize_with(env_file, &cwd).expect("Could not get env path");
    let env_path_str = env_path.to_string_lossy();
    let actual = nu!(
        cwd: &cwd,
        format!("nu --env-config '{}' -c '$nu.env-path'", env_path_str)
    );
    assert_eq!(actual.out, env_path.to_string_lossy().to_string());
}

#[test]
fn use_last_config_path() {
    let config_file = "crates/nu-utils/src/default_files/scaffold_config.nu";
    let env_file = "crates/nu-utils/src/default_files/scaffold_env.nu";

    let cwd = std::env::current_dir().expect("Could not get current working directory");

    let config_path =
        nu_path::canonicalize_with(config_file, &cwd).expect("Could not get config path");
    let config_path_str = config_path.to_string_lossy();
    let actual = nu!(
        cwd: &cwd,
        format!("nu --config non-existing-path --config another-random-path.nu --config '{}' -c '$nu.config-path'", config_path_str)
    );
    assert_eq!(actual.out, config_path.to_string_lossy().to_string());

    let env_path = nu_path::canonicalize_with(env_file, &cwd).expect("Could not get env path");
    let env_path_str = env_path.to_string_lossy();
    let actual = nu!(
        cwd: &cwd,
        format!("nu --env-config non-existing-path --env-config '{}' -c '$nu.env-path'", env_path_str)
    );
    assert_eq!(actual.out, env_path.to_string_lossy().to_string());
}

#[test]
fn test_xdg_config_empty() {
    Playground::setup("xdg_config_empty", |_, playground| {
        playground.with_env("XDG_CONFIG_HOME", "");

        let actual = run(playground, "$nu.default-config-dir");
        let expected = non_xdg_config_dir();
        assert_eq!(actual, adjust_canonicalization(expected));
    });
}

#[test]
fn test_xdg_config_bad() {
    Playground::setup("xdg_config_bad", |_, playground| {
        let xdg_config_home = r#"mn2''6t\/k*((*&^//k//: "#;
        playground.with_env("XDG_CONFIG_HOME", xdg_config_home);

        let actual = run(playground, "$nu.default-config-dir");
        let expected = non_xdg_config_dir();
        assert_eq!(actual, adjust_canonicalization(expected));

        #[cfg(not(windows))]
        {
            let stderr = run_interactive_stderr(xdg_config_home);
            assert!(
                stderr.contains("xdg_config_home_invalid"),
                "stderr was {stderr}"
            );
        }
    });
}

/// Shouldn't complain if XDG_CONFIG_HOME is a symlink
#[test]
#[cfg(not(windows))]
fn test_xdg_config_symlink() {
    Playground::setup("xdg_config_symlink", |_, playground| {
        let config_link = "config_link";

        playground.symlink("real", config_link);

        let stderr = run_interactive_stderr(playground.cwd().join(config_link));
        assert!(
            !stderr.contains("xdg_config_home_invalid"),
            "stderr was {stderr}"
        );
    });
}

#[test]
fn no_config_does_not_load_env_files() {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let cmd = format!(
        r#"
            {nu} -n -c "view files | where filename =~ 'env\\.nu$' | length"
        "#
    );
    let actual = nu!(cmd);

    assert_eq!(actual.out, "0");
}

#[test]
fn no_config_does_not_load_config_files() {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let cmd = format!(
        r#"
            {nu} -n -c "view files | where filename =~ 'config\\.nu$' | length"
        "#
    );
    let actual = nu!(cmd);

    assert_eq!(actual.out, "0");
}

#[test]
fn commandstring_does_not_load_config_files() {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let cmd = format!(
        r#"
            {nu} -c "view files | where filename =~ 'config\\.nu$' | length"
        "#
    );
    let actual = nu!(cmd);

    assert_eq!(actual.out, "0");
}

#[test]
fn commandstring_does_not_load_user_env() {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let cmd = format!(
        r#"
            {nu} -c "view files | where filename =~ '[^_]env\\.nu$' | length"
        "#
    );
    let actual = nu!(cmd);

    assert_eq!(actual.out, "0");
}

#[test]
fn commandstring_loads_default_env() {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let cmd = format!(
        r#"
            {nu} -c "view files | where filename =~ 'default_env\\.nu$' | length"
        "#
    );
    let actual = nu!(cmd);

    assert_eq!(actual.out, "1");
}

#[test]
fn commandstring_populates_config_record() {
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let cmd = format!(
        r#"
            {nu} --no-std-lib -n -c "$env.config.show_banner"
        "#
    );
    let actual = nu!(cmd);

    assert_eq!(actual.out, "true");
}
