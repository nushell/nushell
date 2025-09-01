use nu_test_support::nu;

#[test]
fn capture_errors_works() {
    let actual = nu!("do -c {$env.use}");

    eprintln!("actual.err: {:?}", actual.err);

    assert!(actual.err.contains("column_not_found"));
}

// TODO: need to add tests under display_error.exit_code = true
#[test]
fn capture_errors_works_for_external() {
    let actual = nu!("do -c {nu --testbin fail}");
    assert!(!actual.status.success());
    assert!(!actual.err.contains("exited with code"));
}

// TODO: need to add tests under display_error.exit_code = true
#[test]
fn capture_errors_works_for_external_with_pipeline() {
    let actual = nu!("do -c {nu --testbin fail} | echo `text`");
    assert!(!actual.status.success());
    assert!(!actual.err.contains("exited with code"));
}

// TODO: need to add tests under display_error.exit_code = true
#[test]
fn capture_errors_works_for_external_with_semicolon() {
    let actual = nu!(r#"do -c {nu --testbin fail}; echo `text`"#);
    assert!(!actual.status.success());
    assert!(!actual.err.contains("exited with code"));
}

#[test]
fn do_with_semicolon_break_on_failed_external() {
    let actual = nu!(r#"do { nu --not_exist_flag }; `text`"#);

    assert_eq!(actual.out, "");
}

#[test]
fn ignore_error_should_work_for_external_command() {
    let actual = nu!(r#"do -i { nu --testbin fail 1 }; echo post"#);

    assert_eq!(actual.err, "");
    assert_eq!(actual.out, "post");
}

#[test]
fn ignore_error_works_with_list_stream() {
    let actual = nu!(r#"do -i { ["a", null, "b"] | ansi strip }"#);
    assert!(actual.err.is_empty());
}

#[test]
fn run_closure_with_it_using() {
    let actual = nu!(r#"let x = {let it = 3; $it}; do $x"#);
    assert!(actual.err.is_empty());
    assert_eq!(actual.out, "3");
}

#[test]
fn required_argument_type_checked() {
    let actual = nu!(r#"do {|x: string| $x} 4"#);
    assert!(actual.out.is_empty());
    assert!(actual.err.contains("nu::shell::cant_convert"));
}

#[test]
fn optional_argument_type_checked() {
    let actual = nu!(r#"do {|x?: string| $x} 4"#);
    assert_eq!(actual.out, "");
    assert!(actual.err.contains("nu::shell::cant_convert"));
}
