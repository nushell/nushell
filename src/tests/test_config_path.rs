use nu_test_support::nu;

#[test]
fn test_default_config_path() {
    let config_dir = nu_path::config_dir().expect("Could not get config directory");
    let cwd = std::env::current_dir().expect("Could not get current working directory");

    let config_path = config_dir.join("nushell").join("config.nu");
    let actual = nu!(cwd: &cwd, "$nu.config-path");
    assert_eq!(actual.out, config_path.to_string_lossy().to_string());

    let env_path = config_dir.join("nushell").join("env.nu");
    let actual = nu!(cwd: &cwd, "$nu.env-path");
    assert_eq!(actual.out, env_path.to_string_lossy().to_string());
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
        format!("nu --config {:?} -c '$nu.config-path'", config_path)
    );
    assert_eq!(actual.out, config_path.to_string_lossy().to_string());

    let env_path = nu_path::canonicalize_with(env_file, &cwd).expect("Could not get env path");
    let actual = nu!(
        cwd: &cwd,
        format!("nu --env-config {:?} -c '$nu.env-path'", env_path)
    );
    assert_eq!(actual.out, env_path.to_string_lossy().to_string());
}
