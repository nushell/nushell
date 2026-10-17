use nu_test_support::prelude::*;

#[test]
fn while_sum() -> Result {
    let code = "
        mut total = 0
        mut x = 0
        while $x <= 10 {
            $total += $x
            $x += 1
        }
        $total
    ";

    test().run(code).expect_value_eq(55)
}

#[test]
fn while_doesnt_auto_print_in_each_iteration() -> Result {
    let code = "
        mut total = 0
        while $total < 2 {
            $total += 1
            1
        }
    ";

    test().run(code).expect_value_eq(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn while_break_on_external_failed() -> Result {
    let code = "
        mut total = 0
        while $total < 2 {
            $total += 1
            fail
        }
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
#[deps(TESTBIN_FAIL)]
fn failed_while_should_break_running() -> Result {
    let code = "
        mut total = 0
        while $total < 2 {
            $total += 1
            fail
        }
        3
    ";

    test()
        .run(code)
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

/// `while` is a statement without output, so like `for` it can't be part of a pipeline, also
/// after an environment shorthand.
#[rstest::rstest]
#[case::bare("while false { } | describe")]
#[case::env_shorthand("FOO=1 while false { } | describe")]
fn while_in_pipeline_is_an_error(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::unexpected_keyword")
}

/// A non-bool condition is the same type mismatch in `while` and `if`.
#[test]
fn while_non_bool_condition() -> Result {
    test()
        .run("while 1 { }")
        .expect_error_code_eq("nu::shell::type_mismatch")
}
