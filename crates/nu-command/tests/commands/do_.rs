use nu_test_support::nu;

#[test]
fn do_with_semicolon_break_on_failed_external() {
    let actual = nu!(r#"do { nu --not_exist_flag }; `text`"#);

    assert_eq!(actual.out, "");
}

#[test]
fn ignore_shell_errors_works_for_external_with_semicolon() {
    let actual = nu!(r#"do -s { open asdfasdf.txt }; "text""#);

    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "text");
}

#[test]
fn ignore_program_errors_works_for_external_with_semicolon() {
    let actual = nu!(r#"do -p { nu -n -c 'exit 1' }; "text""#);

    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "text");
}

#[test]
fn ignore_error_should_work_for_external_command() {
    let actual = nu!(r#"do -i { nu --testbin fail asdf }; echo post"#);

    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "post");
}

#[test]
fn ignore_error_works_with_list_stream() {
    let actual = nu!(r#"do -i { ["a", null, "b"] | ansi strip }"#);
    assert!(actual.err.is_empty());
}
