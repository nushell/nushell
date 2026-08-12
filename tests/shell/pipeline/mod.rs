mod commands;

use nu_test_support::prelude::*;

#[test]
fn doesnt_break_on_utf8() -> Result {
    test().run("echo ö").expect_value_eq("ö")
}

#[test]
#[deps(TESTBIN_IECHO)]
fn infinite_output_piped_to_value() -> Result {
    test().run("iecho x | 1").expect_value_eq(1)
}
