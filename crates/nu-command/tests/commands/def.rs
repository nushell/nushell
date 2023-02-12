use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};
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
fn multi_function_check_type() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            def provideStr [r: record] {
                "should_print_this"
            };
            
            def needStr [str: string] {
                echo $str;
            };
            
            def run [r: record] {
                needStr (provideStr $r)
            };
            run {a:b};
        "#
    ));
    assert_eq!(actual.out, "should_print_this");
}

#[test]
fn multi_function_check_type_more_than_one_parameter() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            def provideStr [h1: number,h2: number] {
                "not_number_type"
            };
            
            def needStr [str: string] {
                return $str  
            };
            
            def run [h1: number,h2: number] {
                needStr (provideStr $h1 $h2)
            };
            run 3 5;
    "#
    ));

    assert_eq!(actual.out, "not_number_type");
}
