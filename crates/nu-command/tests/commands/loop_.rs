use nu_test_support::prelude::*;

#[test]
fn loop_doesnt_auto_print_in_each_iteration() -> Result {
    let code = "
        mut total = 0
        loop {
            if $total == 3 {
                break
            } else {
                $total += 1
            }
            1
        }
    ";

    test().run(code).expect_value_eq(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn loop_break_on_external_failed() -> Result {
    let code = "
        mut total = 0
        loop {
            if $total == 3 {
                break
            } else {
                $total += 1
            }
            fail
        }
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn failed_loop_should_break_running() -> Result {
    let code = "
        mut total = 0
        loop {
            if $total == 3 {
                break
            } else {
                $total += 1
            }
            fail
        }
        3
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

/// `loop` is a statement without output, so like `for` it can't be part of a pipeline, also
/// after an environment shorthand.
#[rstest::rstest]
#[case::bare("loop { break } | describe")]
#[case::env_shorthand("FOO=1 loop { break } | describe")]
fn loop_in_pipeline_is_an_error(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::unexpected_keyword")
}
