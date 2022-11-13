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
fn let_doesnt_mutate() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let i = 3; $i = 4
        "#
    ));

    assert!(actual.err.contains("immutable"));
}
