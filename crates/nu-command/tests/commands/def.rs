use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};
use std::fs;

#[test]
fn def_errors_with_multiple_short_flags() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-command [ --long(-l)(-o) ] {}
        "#
    ));

    assert!(actual.err.contains("expected only one short flag"));
}

#[test]
fn def_errors_with_comma_before_alternative_short_flag() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-command [ --long, (-l) ] {}
        "#
    ));

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_comma_before_equals() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-command [ foo, = 1 ] {}
        "#
    ));

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_comma_before_colon() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-command [ foo, : int ] {}
        "#
    ));

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_multiple_colons() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-command [ foo::int ] {}
        "#
    ));

    assert!(actual.err.contains("expected type"));
}

#[ignore = "This error condition is not implemented yet"]
#[test]
fn def_errors_with_multiple_types() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-command [ foo:int:string ] {}
        "#
    ));

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_multiple_commas() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-command [ foo,,bar ] {}
        "#
    ));

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_fails_with_invalid_name() {
    let err_msg = "command name can't be a number, a filesize, or contain a hash # or caret ^";
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            def 1234 = echo "test"
        "#
    ));
    assert!(actual.err.contains(err_msg));

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            def 5gib = echo "test"
        "#
    ));
    assert!(actual.err.contains(err_msg));

    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            def ^foo [] {}
        "#
    ));
    assert!(actual.err.contains(err_msg));
}

#[test]
fn def_with_list() {
    Playground::setup("def_with_list", |dirs, _| {
        let data = r#"
def e [
param: list
] {echo $param};
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "source def_test; e [one] | to json -r"
        );

        assert!(actual.out.contains(r#"one"#));
    })
}

#[test]
fn def_with_default_list() {
    Playground::setup("def_with_default_list", |dirs, _| {
        let data = r#"
def f [
param: list = [one]
] {echo $param};
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "source def_test; f | to json -r"
        );

        assert!(actual.out.contains(r#"["one"]"#));
    })
}

#[test]
fn def_with_paren_params() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def foo (x: int, y: int) { $x + $y }; foo 1 2
        "#
    ));

    assert_eq!(actual.out, "3");
}
