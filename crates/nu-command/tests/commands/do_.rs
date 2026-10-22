use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn capture_errors_works() -> Result {
    test()
        .run("do -c {$env.use}")
        .expect_error_code_eq("nu::shell::column_not_found")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn capture_errors_works_for_external() -> Result {
    let err = test().run("do -c {fail}").expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 1
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn capture_errors_works_for_external_with_pipeline() -> Result {
    let err = test()
        .run("do -c {fail} | echo `text`")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 1
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn capture_errors_works_for_external_with_semicolon() -> Result {
    let err = test()
        .run("do -c {fail}; echo `text`")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::NonZeroExitCode { exit_code, .. } if exit_code.get() == 1
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn do_with_semicolon_break_on_failed_external() -> Result {
    test()
        .run("do { fail }; `text`")
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn ignore_error_should_work_for_external_command() -> Result {
    test()
        .run("do -i { fail 1 }; echo post")
        .expect_value_eq("post")
}

#[test]
fn ignore_error_works_with_list_stream() -> Result {
    let _: Value = test().run(r#"do -i { ["a", null, "b"] | ansi strip }"#)?;
    Ok(())
}

// Regression test for #18732: `do -i` must suppress a non-zero exit code even
// when the external command's output is piped downstream (the exit code is only
// checked once the byte stream is drained, which happens inside `collect` here).
#[test]
#[deps(TESTBIN_FAIL)]
fn ignore_error_works_for_external_piped_downstream() -> Result {
    test().run("do -i { fail 1 } | collect").expect_value_eq("")
}

// Regression test for #18732: the same must hold when the external command's
// output is collected into a value (`let` puts the pipeline in value position).
#[test]
#[deps(TESTBIN_FAIL)]
fn ignore_error_works_for_external_collected_into_value() -> Result {
    test()
        .run("let x = do -i { fail 1 }; $x")
        .expect_value_eq("")
}

#[test]
fn run_closure_with_do_using() -> Result {
    test()
        .run("let x = {let var = 3; $var}; do $x")
        .expect_value_eq(3)
}

#[test]
fn required_argument_type_checked() -> Result {
    test()
        .run("do {|x: string| $x} 4")
        .expect_error_code_eq("nu::shell::cant_convert")
}

#[test]
fn optional_argument_type_checked() -> Result {
    test()
        .run("do {|x?: string| $x} 4")
        .expect_error_code_eq("nu::shell::cant_convert")
}
