use nu_test_support::nu;

#[test]
fn register() {
    let out = nu!("register crates/nu_plugin_nu_example/nu_plugin_nu_example.nu");
    assert!(out.status.success());
    assert!(out.out.trim().is_empty());
    assert!(out.err.trim().is_empty());
}

#[test]
fn call() {
    let out = nu!(r#"
        register crates/nu_plugin_nu_example/nu_plugin_nu_example.nu
        nu_plugin_nu_example 4242 teststring
    "#);
    assert!(out.status.success());

    assert!(out.err.contains("name: nu_plugin_nu_example"));
    assert!(out.err.contains("4242"));
    assert!(out.err.contains("teststring"));

    assert!(out.out.contains("one"));
    assert!(out.out.contains("two"));
    assert!(out.out.contains("three"));
}
