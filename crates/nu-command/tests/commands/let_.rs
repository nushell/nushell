use nu_test_support::{nu, pipeline};

#[test]
fn let_parse_error() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let in = 3
        "#
    ));

    assert!(actual
        .err
        .contains("'in' is the name of a builtin Nushell variable"));
}

#[test]
fn let_parse_error_when_name_is_number() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let 123 = 3
        "#
    ));

    assert!(actual
        .err
        .contains("variable name can't contain spaces or quotes or be a number"));
}
