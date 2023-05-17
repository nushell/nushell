use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::fs;

#[test]
fn def_with_comment() {
    Playground::setup("def_with_comment", |dirs, _| {
        let data = r#"
#My echo
export def e [arg] {echo $arg}
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "use def_test e; help e | to json -r"
        );

        assert!(actual.out.contains("My echo\\n\\n"));
    });
}

#[test]
fn def_with_param_comment() {
    Playground::setup("def_with_param_comment", |dirs, _| {
        let data = r#"
export def e [
param:string #My cool attractive param
] {echo $param};
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "use def_test e; help e"
        );

        assert!(actual.out.contains(r#"My cool attractive param"#));
    })
}

#[test]
fn def_errors_with_no_space_between_params_and_name_1() {
    let actual = nu!("def test-command[] {}");

    assert!(actual.err.contains("expected space"));
}

#[test]
fn def_errors_with_no_space_between_params_and_name_2() {
    let actual = nu!("def-env test-command() {}");

    assert!(actual.err.contains("expected space"));
}

#[test]
fn def_errors_with_multiple_short_flags() {
    let actual = nu!("def test-command [ --long(-l)(-o) ] {}");

    assert!(actual.err.contains("expected only one short flag"));
}

#[test]
fn def_errors_with_comma_before_alternative_short_flag() {
    let actual = nu!("def test-command [ --long, (-l) ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_comma_before_equals() {
    let actual = nu!("def test-command [ foo, = 1 ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_comma_before_colon() {
    let actual = nu!("def test-command [ foo, : int ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_multiple_colons() {
    let actual = nu!("def test-command [ foo::int ] {}");
    assert!(actual.err.contains("expected type"));
}

#[ignore = "This error condition is not implemented yet"]
#[test]
fn def_errors_with_multiple_types() {
    let actual = nu!("def test-command [ foo:int:string ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_multiple_commas() {
    let actual = nu!("def test-command [ foo,,bar ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_fails_with_invalid_name() {
    let err_msg = "command name can't be a number, a filesize, or contain a hash # or caret ^";
    let actual = nu!(r#"def 1234 = echo "test""#);
    assert!(actual.err.contains(err_msg));

    let actual = nu!(r#"def 5gib = echo "test""#);
    assert!(actual.err.contains(err_msg));

    let actual = nu!(r#"def ^foo [] {}"#);
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
    let actual = nu!("def foo (x: int, y: int) { $x + $y }; foo 1 2");

    assert_eq!(actual.out, "3");
}

#[test]
fn extern_with_block() {
    let actual =
        nu!("extern foo [...rest] { print ($rest | str join ',' ) }; foo --bar baz -- -q -u -x");

    assert_eq!(actual.out, "--bar,baz,--,-q,-u,-x");
}
