use nu_test_support::{nu, pipeline};

#[test]
fn const_variable() {
    let actual = nu!(
        cwd: ".",
        pipeline(
        r#"
            const name = "nushell";
            $name == "nushell"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn const_with_builtin_var() {
    let actual = nu!(
        cwd: ".",
        pipeline(
        r#"
            const in = "nushell"
        "#
    ));

    assert!(actual
        .err
        .contains("'in' is the name of a builtin Nushell variable"));
}

#[test]
fn const_with_non_const_val() {
    let actual = nu!(
        cwd: ".",
        pipeline(
        r#"
            const write_test = random bool
        "#
    ));

    assert!(actual.err.contains("Value not a constant"));
}

#[test]
fn const_in_pipeline() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        const a = 42 | math sin 
        "#
    ));

    assert!(actual.err.contains("const statement used in pipeline"));
}
