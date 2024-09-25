use nu_test_support::nu_with_plugins;

#[test]
fn call_to_json() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            [42] | example call-decl 'to json' {indent: 4}
        "#
    );
    assert!(result.status.success());
    // newlines are removed from test output
    assert_eq!("[    42]", result.out);
}

#[test]
fn call_reduce() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            [1 2 3] | example call-decl 'reduce' {fold: 10} { |it, acc| $it + $acc }
        "#
    );
    assert!(result.status.success());
    assert_eq!("16", result.out);
}

#[test]
fn call_scope_variables() {
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            let test_var = 10
            example call-decl 'scope variables' | where name == '$test_var' | length
        "#
    );
    assert!(result.status.success());
    assert_eq!("1", result.out);
}
