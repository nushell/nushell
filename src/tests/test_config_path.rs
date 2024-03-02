use nu_test_support::nu;
use pretty_assertions::assert_eq;
use std::fs;
use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use nu_test_support::playground::Playground;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::PathBuf;

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

/// Make the config directory a symlink that points to a temporary folder.
/// Returns the path to the `nushell` config folder inside, via the symlink.
///
/// Need to figure out how to change config directory on Windows.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn setup_fake_config(playground: &mut Playground) -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let config_dir = "config";
        let config_link = "config_link";
        playground.mkdir(&format!("{config_dir}/nushell"));
        playground.symlink(config_dir, config_link);
        playground.with_env(
            "XDG_CONFIG_HOME",
            &playground.cwd().join(config_link).display().to_string(),
        );
        Path::new(config_link).join("nushell")
    }

    #[cfg(target_os = "macos")]
    {
        let fake_home = "fake_home";
        let home_link = "home_link";
        let dir_end = "fake-home/Library/Application\\ Support/nushell";
        playground.mkdir(&format!("{fake_home}/{dir_end}"));
        playground.symlink(fake_home, home_link);
        playground.with_env(
            "HOME",
            &playground.cwd().join(home_link).display().to_string(),
        );
        PathBuf::from(home_link).join(dir_end)
    }
}

fn test_config_path_helper() {
    let config_dir = nu_path::config_dir().expect("Could not get config directory");
    let config_dir_nushell = config_dir.join("nushell");
    // Create the config dir folder structure if it does not already exist
    if !config_dir_nushell.exists() {
        let _ = fs::create_dir_all(&config_dir_nushell);
    }
    let cwd = std::env::current_dir().expect("Could not get current working directory");

    let config_dir_nushell =
        std::fs::canonicalize(&config_dir_nushell).expect("canonicalize config dir failed");
    let actual = nu!(cwd: &cwd, "$nu.default-config-dir");
    assert_eq!(actual.out, adjust_canonicalization(&config_dir_nushell));

    let config_path = config_dir_nushell.join("config.nu");
    // We use canonicalize here in case the config or env is symlinked since $nu.config-path is returning the canonicalized path in #8653
    let canon_config_path =
        adjust_canonicalization(std::fs::canonicalize(&config_path).unwrap_or(config_path));
    let actual = nu!(cwd: &cwd, "$nu.config-path");
    assert_eq!(actual.out, canon_config_path);

    let env_path = config_dir_nushell.join("env.nu");
    let canon_env_path =
        adjust_canonicalization(std::fs::canonicalize(&env_path).unwrap_or(env_path));
    let actual = nu!(cwd: &cwd, "$nu.env-path");
    assert_eq!(actual.out, canon_env_path);

    let history_path = config_dir_nushell.join("history.txt");
    let canon_history_path =
        adjust_canonicalization(std::fs::canonicalize(&history_path).unwrap_or(history_path));
    let actual = nu!(cwd: &cwd, "$nu.history-path");
    assert_eq!(actual.out, canon_history_path);

    let login_path = config_dir_nushell.join("login.nu");
    let canon_login_path =
        adjust_canonicalization(std::fs::canonicalize(&login_path).unwrap_or(login_path));
    let actual = nu!(cwd: &cwd, "$nu.loginshell-path");
    assert_eq!(actual.out, canon_login_path);

    #[cfg(feature = "plugin")]
    {
        let plugin_path = config_dir_nushell.join("plugin.nu");
        let canon_plugin_path =
            adjust_canonicalization(std::fs::canonicalize(&plugin_path).unwrap_or(plugin_path));
        let actual = nu!(cwd: &cwd, "$nu.plugin-path");
        assert_eq!(actual.out, canon_plugin_path);
    }
}

#[test]
fn test_default_config_path() {
    test_config_path_helper();
}

/// Make the config folder a symlink to a temporary folder without any config files
/// and see if the config files' paths are properly canonicalized
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn test_default_symlinked_config_path_empty() {
    Playground::setup("symlinked_empty_config_dir", |_, playground| {
        let _ = setup_fake_config(playground);

        test_config_path_helper();
    });
}

/// Like [[test_default_symlinked_config_path_empty]], but fill the temporary folder
/// with broken symlinks and see if they're properly canonicalized
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn test_default_symlink_config_path_broken_symlink_config_files() {
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
                "plugin.nu",
            ] {
                playground.symlink(
                    format!("fake/{config_file}"),
                    fake_config_dir_nushell.join(config_file),
                );
            }

            test_config_path_helper();
        },
    );
}

/// Like [[test_default_symlinked_config_path_empty]], but fill the temporary folder
/// with working symlinks to empty files and see if they're properly canonicalized
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn test_default_config_path_symlinked_config_files() {
    use std::fs::File;

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
                "plugin.nu",
            ] {
                let empty_file = playground.cwd().join(format!("empty-{config_file}"));
                File::create(&empty_file).unwrap();
                playground.symlink(empty_file, fake_config_dir_nushell.join(config_file));
            }

            test_config_path_helper();
        },
    );
}

#[test]
fn test_alternate_config_path() {
    let config_file = "crates/nu-utils/src/sample_config/default_config.nu";
    let env_file = "crates/nu-utils/src/sample_config/default_env.nu";

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
