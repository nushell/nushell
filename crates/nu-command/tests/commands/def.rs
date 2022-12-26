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
