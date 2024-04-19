use nu_test_support::nu_with_plugins;

#[test]
fn plugin_add_then_restart_nu() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        if cfg!(windows) {
            r#"
                plugin add target\debug\nu_plugin_example.exe
                ^$nu.current-exe --config $nu.config-path --env-config $nu.env-path --plugin-config $nu.plugin-path --commands 'plugin list | get name | to json --raw'
            "#
        } else {
            r#"
                plugin add target/debug/nu_plugin_example
                ^$nu.current-exe --config $nu.config-path --env-config $nu.env-path --plugin-config $nu.plugin-path --commands 'plugin list | get name | to json --raw'
            "#
        }
    );
    assert!(result.status.success());
    assert_eq!(r#"["example"]"#, result.out);
}

#[test]
fn plugin_rm_then_restart_nu() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            plugin rm example
            ^$nu.current-exe --config $nu.config-path --env-config $nu.env-path --plugin-config $nu.plugin-path --commands 'plugin list | get name | to json --raw'
        "#
    );
    assert!(result.status.success());
    assert_eq!(r#"[]"#, result.out);
}
