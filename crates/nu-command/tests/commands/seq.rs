use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn float_in_seq_leads_to_lists_of_floats() -> Result {
    test()
        .run("seq 1.0 0.5 6 | describe")
        .expect_value_eq("list<float> (stream)")
}

#[test]
fn ints_in_seq_leads_to_lists_of_ints() -> Result {
    test()
        .run("seq 1 2 6 | describe")
        .expect_value_eq("list<int> (stream)")
}

#[rstest]
#[case::non_terminating("seq 5 0 5")]
#[case::empty_range("seq 1 0 5")]
#[case::float("seq 1.0 0.0 5.0")]
fn zero_increment_is_rejected(#[case] code: &str) -> Result {
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::IncorrectValue { msg, .. } if msg == "Increment can't be 0");
    Ok(())
}

#[rstest]
#[case("seq 0 1 10 NaN")]
#[case("seq 0 1 NaN 10")]
#[case("seq 0 1 NaN")]
#[case("seq 0 NaN 10")]
#[case("seq NaN 1 10")]
fn nan_is_rejected(#[case] code: &str) -> Result {
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::IncorrectValue { msg, .. } if msg == "NaN values are not allowed");
    Ok(())
}

#[rstest]
#[case("seq 0 1 inf 10")]
#[case("seq 0 inf 10")]
#[case("seq inf 1 10")]
#[case("seq inf 1")]
fn non_final_infinite_is_rejected(#[case] code: &str) -> Result {
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::IncorrectValue { msg, .. } if msg == "Non-final infinite values are not allowed");
    Ok(())
}

#[rstest]
#[case("seq 0 1 10 inf", [0.0, 1.0, 2.0])]
#[case("seq 0 1 inf", [0.0, 1.0, 2.0])]
#[case("seq 0 inf", [0.0, 1.0, 2.0])]
#[case("seq inf", [f64::INFINITY, f64::INFINITY, f64::INFINITY])]
fn final_infinite_is_allowed(#[case] code: &str, #[case] expected: [f64; 3]) -> Result {
    let code = format!("{} | first 3", code);
    test().run(code).expect_value_eq(expected)
}

#[test]
fn int_sequence_at_max_does_not_panic() -> Result {
    // Advancing past i64::MAX previously panicked with "attempt to add with
    // overflow"; the final in-range value must still be emitted and the
    // sequence must end cleanly.
    test()
        .run("seq 9223372036854775807 9223372036854775807")
        .expect_value_eq([i64::MAX])
}
