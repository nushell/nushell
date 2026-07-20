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
            print 1
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
