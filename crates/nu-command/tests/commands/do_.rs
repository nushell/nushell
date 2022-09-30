use nu_test_support::{nu, pipeline};

#[test]
fn capture_errors_works() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        do -c {$env.use} | describe
        "#
    ));

    assert_eq!(actual.out, "error");
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
