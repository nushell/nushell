use nu_test_support::nu_with_plugins;

#[test]
fn get_env_by_name() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            $env.FOO = bar
            nu-example-env FOO | print
            $env.FOO = baz
            nu-example-env FOO | print
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
        "$env.BAZ = foo; nu-example-env | get BAZ"
    );
    assert!(result.status.success());
    assert_eq!("foo", result.out);
}
