use nu_test_support::nu_with_plugins;

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
fn some() {
    let actual = nu_with_plugins!(
        cwd: "tests",
        plugin: ("nu_plugin_example"),
        r#"
            $env.config = {
                plugins: {
                    example: {
                        path: "some/path",
                        nested: {
                            bool: true,
                            string: "Hello Example!"
                        }
                    }
                }
            }
            example config
        "#
    );

    assert!(actual.out.contains("some/path"));
    assert!(actual.out.contains("Hello Example!"));
}
