use nu_test_support::{nu, pipeline};

#[ignore = "TODO?: Aliasing parser keywords does not work anymore"]
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

#[ignore = "TODO?: Aliasing parser keywords does not work anymore"]
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

#[ignore = "TODO?: Aliasing parser keywords does not work anymore"]
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
    let err_msg = "name can't be a number, a filesize, or contain a hash # or caret ^";
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias 1234 = echo "test"
        "#
    ));
    assert!(actual.err.contains(err_msg));

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias 5gib = echo "test"
        "#
    ));
    assert!(actual.err.contains(err_msg));

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias "te#t" = echo "test"
        "#
    ));
    assert!(actual.err.contains(err_msg));

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias ^foo = echo "bar"
        "#
    ));
    assert!(actual.err.contains(err_msg));
}

#[test]
fn cant_alias_keyword() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias ou = let
        "#
    ));
    assert!(actual.err.contains("cant_alias_keyword"));
}
