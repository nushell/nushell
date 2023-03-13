use nu_test_support::{nu, pipeline};

#[test]
fn let_with_builtin_var() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let in = 3
        "#
    ));

    assert!(actual
        .err
        .contains("'in' is the name of a builtin Nushell variable."));
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

#[test]
fn let_with_external_failed() {
    let actual = nu!(
        cwd: ".",
        pipeline(r#"let x = nu --testbin outcome_err "aa"; echo fail"#)
    );

    assert!(!actual.out.contains("fail"));
}

#[test]
fn let_in_pipeline() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let a = 42 | math sin 
        "#
    ));

    assert!(actual.err.contains("let statement used in pipeline"))
}
