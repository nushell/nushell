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
    let err_msg = "alias name can't be a number, a filesize, or contain a hash # or caret ^";
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
            alias ^foo = "bar"
        "#
    ));
    assert!(actual.err.contains(err_msg));
}

#[test]
fn alias_alone_lists_aliases() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias a = 3; alias
        "#
    ));
    assert!(actual.out.contains("name") && actual.out.contains("expansion"));
}

#[test]
fn alias_short_attr1() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias t = if not true { ls } else { echo "should_type_this"}; t
        "#
    ));

    assert_eq!(actual.out, "should_type_this");
}

#[test]
fn alias_short_attr2() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias vi = if not ( [] | is-empty) { echo "first_one" } else if not ( [1] | is-empty) { echo "second_one" } ; vi;
        "#
    ));

    assert!(actual.err.is_empty());
    assert!(actual.out.contains("second_one"));

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            alias v = if not false { echo "first_one" } else { echo "second_one" }; v;
        "#
    ));

    assert!(actual.out.contains("first_one"));
}
