use nu_test_support::nu_with_plugins;

#[test]
fn plugin_add_then_restart_nu() {
    let bins_path = nu_test_support::fs::binaries();
    let example_plugin_path = nu_path::canonicalize_with(
        if cfg!(windows) {
            "nu_plugin_example.exe"
        } else {
            "nu_plugin_example"
        },
        bins_path,
    )
    .expect("nu_plugin_example not found");
    let result = nu_with_plugins!(
        cwd: ".",
        plugins: [],
        &format!("
            plugin add '{}'
            ^$nu.current-exe --config $nu.config-path --env-config $nu.env-path --plugin-config $nu.plugin-path --commands 'plugin list | get name | to json --raw'
        ", example_plugin_path.display())
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
