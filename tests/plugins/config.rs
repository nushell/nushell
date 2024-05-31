use nu_test_support::nu_with_plugins;

#[test]
fn closure() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_example"),
        r#"
            $env.env_value = "value from env"

            $env.config = {
                plugins: {
                    example: {||
                        $env.env_value
                    }
                }
            }
            example config
        "#
    );

    assert!(actual.out.contains("value from env"));
}

#[test]
fn none() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_example"),
        "example config"
    );

    assert!(actual.err.contains("No config sent"));
}

#[test]
fn record() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_example"),
        r#"
            $env.config = {
                plugins: {
                    example: {
                        key1: "value"
                        key2: "other"
                    }
                }
            }
            example config
        "#
    );

    assert!(actual.out.contains("value"));
    assert!(actual.out.contains("other"));
}
