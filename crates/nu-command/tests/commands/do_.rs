use nu_test_support::{nu, pipeline};

#[test]
fn capture_errors_works() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {$env.use}
        "#
    ));

    assert!(actual.err.contains("column_not_found"));
}

#[test]
fn capture_errors_works_for_external() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {nu --testbin fail}
        "#
    ));
    assert!(actual.err.contains("External command runs to failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn capture_errors_works_for_external_with_pipeline() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {nu --testbin fail} | echo `text`
        "#
    ));
    assert!(actual.err.contains("External command runs to failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn capture_errors_works_for_external_with_semicolon() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {nu --testbin fail}; echo `text`
        "#
    ));
    assert!(actual.err.contains("External command runs to failed"));
    assert_eq!(actual.out, "");
}

#[test]
fn do_with_semicolon_break_on_failed_external() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do { nu --not_exist_flag }; `text`
        "#
    ));

    assert_eq!(actual.out, "");
}
