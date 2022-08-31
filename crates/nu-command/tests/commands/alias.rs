use nu_test_support::{nu, pipeline};

#[test]
fn alias_simple() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            alias bar = use sample_def.nu greet;
            bar;
            greet
        "#
    ));

    assert_eq!(actual.out, "hello");
}

#[test]
fn alias_hiding_1() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            overlay use ./activate-foo.nu;
            $nu.scope.aliases | find deactivate-foo | length
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn alias_hiding_2() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            overlay use ./activate-foo.nu;
            deactivate-foo;
            $nu.scope.aliases | find deactivate-foo | length
        "#
    ));

    assert_eq!(actual.out, "0");
}
