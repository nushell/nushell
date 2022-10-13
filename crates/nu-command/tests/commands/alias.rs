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

#[test]
fn alias_fails_with_invalid_name() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            alias 1234 = echo "test"   
        "#
    ));
    assert!(actual
        .err
        .contains("alias name can't be a number or a filesize"));
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            alias 5gib = echo "test"   
        "#
    ));
    assert!(actual
        .err
        .contains("alias name can't be a number or a filesize"));
}

#[test]
fn alias_alone_lists_aliases() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias a = 3; alias
        "#
    ));
    assert!(actual.out.contains("alias") && actual.out.contains("expansion"));
}
