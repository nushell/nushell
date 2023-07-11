use nu_test_support::{nu, pipeline};

#[test]
fn let_name_builtin_var() {
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

#[test]
fn let_takes_pipeline() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let x = "hello world" | str length; print $x
        "#
    ));

    assert_eq!(actual.out, "11");
}

#[test]
fn let_pipeline_allows_in() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def foo [] { let x = $in | str length; print ($x + 10) }; "hello world" | foo
        "#
    ));

    assert_eq!(actual.out, "21");
}

#[test]
fn let_with_no_spaces_1() {
    let actual = nu!(
        cwd: ".",
        pipeline("let x=4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_2() {
    let actual = nu!(
        cwd: ".",
        pipeline("let x =4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_3() {
    let actual = nu!(
        cwd: ".",
        pipeline("let x= 4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_4() {
    let actual = nu!(
        cwd: ".",
        pipeline("let x: int= 4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_5() {
    let actual = nu!(
        cwd: ".",
        pipeline("let x:int= 4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_no_spaces_6() {
    let actual = nu!(
        cwd: ".",
        pipeline("let x : int = 4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn let_with_complex_type() {
    let actual = nu!(
        cwd: ".",
        pipeline("let x: record<name: string> = { name: 'nushell' }; $x")
    );

    assert_eq!(actual.out, "4");
}

#[ignore]
#[test]
fn let_with_external_failed() {
    // FIXME: this test hasn't run successfully for a long time. We should
    // bring it back to life at some point.
    let actual = nu!(
        cwd: ".",
        pipeline(r#"let x = nu --testbin outcome_err "aa"; echo fail"#)
    );

    assert!(!actual.out.contains("fail"));
}
