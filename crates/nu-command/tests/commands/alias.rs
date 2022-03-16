use nu_test_support::{nu, pipeline};

#[test]
fn alias_simple() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        alias bar = source sample_def.nu; bar; greet
        "#
    ));

    assert_eq!(actual.out, "hello");
}

#[test]
fn alias_hiding1() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        source ./activate-foo.nu;
        $nu.scope.aliases | find deactivate-foo | length
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn alias_hiding2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        source ./activate-foo.nu;
        deactivate-foo;
        $nu.scope.aliases | find deactivate-foo | length
        "#
    ));

    assert_eq!(actual.out, "0");
}
