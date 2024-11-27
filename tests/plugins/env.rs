use nu_test_support::nu_with_plugins;

#[test]
fn get_env_by_name() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            $env.FOO = 'bar'
            example env FOO | print
            $env.FOO = 'baz'
            example env FOO | print
        "#
    );
    assert!(result.status.success());
    assert_eq!("barbaz", result.out);
}

#[test]
fn get_envs() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        "$env.BAZ = 'foo'; example env | get BAZ"
    );
    assert!(result.status.success());
    assert_eq!("foo", result.out);
}

#[test]
fn get_current_dir() {
    let cwd = std::env::current_dir()
        .expect("failed to get current dir")
        .join("tests")
        .to_string_lossy()
        .into_owned();
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        "cd tests; example env --cwd"
    );
    assert!(result.status.success());
    #[cfg(not(windows))]
    assert_eq!(cwd, result.out);
    #[cfg(windows)]
    {
        // cwd == r"e:\Study\Nushell", while result.out == r"E:\Study\Nushell"
        assert_eq!(
            cwd.chars().next().unwrap().to_ascii_uppercase(),
            result.out.chars().next().unwrap().to_ascii_uppercase()
        );
        assert_eq!(cwd[1..], result.out[1..]);
    }
}

#[test]
fn set_env() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        "example env NUSHELL_OPINION --set=rocks; $env.NUSHELL_OPINION"
    );
    assert!(result.status.success());
    assert_eq!("rocks", result.out);
}
