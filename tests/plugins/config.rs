use nu_test_support::nu_with_plugins;

#[test]
fn config() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_config"),
        r#"
            $env.config = {
                plugins: {
                    config: {
                        key1: "value"
                        key2: "other"
                    }
                }
            }
            nu-plugin-config
        "#
    );

    assert!(actual.out.contains("value"));
    assert!(actual.out.contains("other"));
}

#[test]
fn no_config() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_config"),
        "nu-plugin-config"
    );

    assert!(actual.err.contains("No config sent"));
}
