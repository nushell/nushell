use nu_path::{AbsolutePath, AbsolutePathBuf, Path};
use nu_test_support::nu;
use nu_test_support::playground::{Executable, Playground};
use pretty_assertions::assert_eq;
use std::fs::{self, File};

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

/// Make the config directory a symlink that points to a temporary folder, and also makes
/// the nushell directory inside a symlink.
/// Returns the path to the `nushell` config folder inside, via the symlink.
fn setup_fake_config(playground: &mut Playground) -> AbsolutePathBuf {
    let config_real = "config_real";
    let config_link = "config_link";
    let nushell_real = "nushell_real";
    let nushell_link = Path::new(config_real)
        .join("nushell")
        .into_os_string()
        .into_string()
        .unwrap();

    let config_home = playground.cwd().join(config_link);

    playground.mkdir(nushell_real);
    playground.mkdir(config_real);
    playground.symlink(nushell_real, &nushell_link);
    playground.symlink(config_real, config_link);
    playground.with_env("XDG_CONFIG_HOME", config_home.to_str().unwrap());

    let path = config_home.join("nushell");
    path.canonicalize().map(Into::into).unwrap_or(path)
}

fn run(playground: &mut Playground, command: &str) -> String {
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
    let child_output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{:?} -i -c 'echo $nu.is-interactive'",
            nu_test_support::fs::executable_path()
        ))
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

    let config_dir_nushell = config_dir_nushell
        .canonicalize()
        .expect("canonicalize config dir failed");
    let actual = run(playground, "$nu.default-config-dir");
    assert_eq!(actual, adjust_canonicalization(&config_dir_nushell));

    let config_path = config_dir_nushell.join("config.nu");
    // We use canonicalize here in case the config or env is symlinked since $nu.config-path is returning the canonicalized path in #8653
    let canon_config_path =
        adjust_canonicalization(std::fs::canonicalize(&config_path).unwrap_or(config_path.into()));
    let actual = run(playground, "$nu.config-path");
    assert_eq!(actual, canon_config_path);

    let env_path = config_dir_nushell.join("env.nu");
    let canon_env_path =
        adjust_canonicalization(std::fs::canonicalize(&env_path).unwrap_or(env_path.into()));
    let actual = run(playground, "$nu.env-path");
    assert_eq!(actual, canon_env_path);

    let history_path = config_dir_nushell.join("history.txt");
    let canon_history_path = adjust_canonicalization(
        std::fs::canonicalize(&history_path).unwrap_or(history_path.into()),
    );
    let actual = run(playground, "$nu.history-path");
    assert_eq!(actual, canon_history_path);

    let login_path = config_dir_nushell.join("login.nu");
    let canon_login_path =
        adjust_canonicalization(std::fs::canonicalize(&login_path).unwrap_or(login_path.into()));
    let actual = run(playground, "$nu.loginshell-path");
    assert_eq!(actual, canon_login_path);

    #[cfg(feature = "plugin")]
    {
        let plugin_path = config_dir_nushell.join("plugin.msgpackz");
        let canon_plugin_path = adjust_canonicalization(
            std::fs::canonicalize(&plugin_path).unwrap_or(plugin_path.into()),
        );
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

/// Make the config folder a symlink to a temporary folder without any config files
/// and see if the config files' paths are properly canonicalized
#[test]
fn test_default_symlinked_config_path_empty() {
    Playground::setup("symlinked_empty_config_dir", |_, playground| {
        let config_dir_nushell = setup_fake_config(playground);
        test_config_path_helper(playground, config_dir_nushell);
    });
}

/// Like [`test_default_symlinked_config_path_empty`], but fill the temporary folder
/// with broken symlinks and see if they're properly canonicalized
#[test]
fn test_default_symlink_config_path_broken_symlink_config_files() {
    Playground::setup(
        "symlinked_cfg_dir_with_symlinked_cfg_files_broken",
        |_, playground| {
            let fake_config_dir_nushell = setup_fake_config(playground);

            let fake_dir = "fake";
            playground.mkdir(fake_dir);
            let fake_dir = Path::new(fake_dir);

            for config_file in [
                "config.nu",
                "env.nu",
                "history.txt",
                "history.sqlite3",
                "login.nu",
                "plugin.msgpackz",
            ] {
                let fake_file = fake_dir.join(config_file);
                File::create(playground.cwd().join(&fake_file)).unwrap();

                playground.symlink(&fake_file, fake_config_dir_nushell.join(config_file));
            }

            // Windows doesn't allow creating a symlink without the file existing,
            // so we first create original files for the symlinks, then delete them
            // to break the symlinks
            std::fs::remove_dir_all(playground.cwd().join(fake_dir)).unwrap();

            test_config_path_helper(playground, fake_config_dir_nushell);
        },
    );
}

/// Like [`test_default_symlinked_config_path_empty`], but fill the temporary folder
/// with working symlinks to empty files and see if they're properly canonicalized
#[test]
fn test_default_config_path_symlinked_config_files() {
    Playground::setup(
        "symlinked_cfg_dir_with_symlinked_cfg_files",
        |_, playground| {
            let fake_config_dir_nushell = setup_fake_config(playground);

            for config_file in [
                "config.nu",
                "env.nu",
                "history.txt",
                "history.sqlite3",
                "login.nu",
                "plugin.msgpackz",
            ] {
                let empty_file = playground.cwd().join(format!("empty-{config_file}"));
                File::create(&empty_file).unwrap();
                playground.symlink(empty_file, fake_config_dir_nushell.join(config_file));
            }

            test_config_path_helper(playground, fake_config_dir_nushell);
        },
    );
}

#[test]
fn test_alternate_config_path() {
    let config_file = "crates/nu-utils/src/default_files/scaffold_config.nu";
    let env_file = "crates/nu-utils/src/default_files/scaffold_env.nu";

    let cwd = std::env::current_dir().expect("Could not get current working directory");

    let config_path =
        nu_path::canonicalize_with(config_file, &cwd).expect("Could not get config path");
    let actual = nu!(
        cwd: &cwd,
        format!("nu --config {config_path:?} -c '$nu.config-path'")
    );
    assert_eq!(actual.out, config_path.to_string_lossy().to_string());

    let env_path = nu_path::canonicalize_with(env_file, &cwd).expect("Could not get env path");
    let actual = nu!(
        cwd: &cwd,
        format!("nu --env-config {env_path:?} -c '$nu.env-path'")
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
    let actual = nu!(
        cwd: &cwd,
        format!("nu --config non-existing-path --config another-random-path.nu --config {config_path:?} -c '$nu.config-path'")
    );
    assert_eq!(actual.out, config_path.to_string_lossy().to_string());

    let env_path = nu_path::canonicalize_with(env_file, &cwd).expect("Could not get env path");
    let actual = nu!(
        cwd: &cwd,
        format!("nu --env-config non-existing-path --env-config {env_path:?} -c '$nu.env-path'")
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
