use nu_test_support::prelude::*;
use rstest::rstest;

// The first row's source is the whole `debug profile` call, which spans two lines here.
#[rstest]
#[case::first_line_only("debug profile {\n1 } | first | get source", "debug profile { ...")]
#[case::every_line(
    "debug profile --expand-source {\n1 } | first | get source",
    "debug profile --expand-source {\n1 }"
)]
fn source_of_a_multiline_call(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

// A profiler left running would slow down everything evaluated after the failure.
#[test]
fn failing_closure_stops_the_profiler() -> Result {
    let mut tester = test();
    tester
        .run("debug profile { error make { msg: boom } }")
        .expect_shell_error()?;

    assert!(!tester.engine_state.is_debugging());
    Ok(())
}
