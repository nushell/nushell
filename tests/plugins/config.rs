use nu_test_support::nu_with_plugins;

#[test]
fn closure() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_config"),
        r#"
            $env.env_value = "value from env"

            $env.config = {
                plugins: {
                    config: {||
                        $env.env_value
                    }
                }
            }
            nu-plugin-config
        "#
    );

    assert!(actual.out.contains("value from env"));
}

#[test]
fn none() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_config"),
        "nu-plugin-config"
    );

    assert!(actual.err.contains("No config sent"));
}

#[test]
fn record() {
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
