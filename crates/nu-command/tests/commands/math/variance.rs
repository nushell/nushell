use nu_test_support::prelude::*;

#[test]
fn const_variance() -> Result {
    test()
        .run("const VAR = [1 2 3 4 5] | math variance; $VAR")
        .expect_value_eq(2.0)
}

#[test]
fn can_variance_range() -> Result {
    let expected: Value = test().run("[0 1 2 3 4 5] | math variance")?;
    test().run("0..5 | math variance").expect_value_eq(expected)
}

#[test]
fn cannot_variance_infinite_range() -> Result {
    let outcome = test().run("0.. | math variance").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn sample_variance_empty_is_error_not_panic() -> Result {
    let err = test()
        .run("[] | math variance --sample")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::UnsupportedInput { .. }));
    Ok(())
}

#[test]
fn sample_variance_single_value_is_error() -> Result {
    let err = test()
        .run("[1] | math variance --sample")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::UnsupportedInput { .. }));
    Ok(())
}

#[test]
fn variance_duration_returns_number() -> Result {
    // Population variance of [1sec, 3sec] is 1e18 (nanoseconds squared).
    test()
        .run("[1sec 3sec] | math variance")
        .expect_value_eq(1_000_000_000_000_000_000.0)
}

#[test]
fn variance_filesize_returns_number_in_bytes_squared() -> Result {
    // 1KB=1000B, 3KB=3000B → population variance 1_000_000 (B²), not 1 (KB²).
    test()
        .run("[1KB 3KB] | math variance")
        .expect_value_eq(1_000_000.0)
}

#[test]
fn variance_large_close_floats() -> Result {
    // Regression: the single-pass formula returned -32.0 here (negative variance).
    test()
        .run("[506250000.0 506250001.0] | math variance")
        .expect_value_eq(0.25)
}

#[test]
fn variance_large_close_ints() -> Result {
    // Regression: the single-pass formula returned 0.0 here.
    test()
        .run("[1000000000 1000000001 1000000002] | math variance")
        .expect_value_eq(2.0 / 3.0)
}

#[test]
fn sample_variance_large_close_floats() -> Result {
    // Same input, `--sample` divides by n - 1.
    test()
        .run("[506250000.0 506250001.0] | math variance --sample")
        .expect_value_eq(0.5)
}

#[test]
fn variance_of_ints_is_still_float() -> Result {
    // The two-pass rewrite must not change the result type.
    test()
        .run("[1 2 3 4 5] | math variance | describe")
        .expect_value_eq("float")
}

#[test]
fn variance_ints_whose_square_overflows_i64() -> Result {
    // The single-pass path used to fail with OperatorOverflow here because it
    // multiplied the values; they are summed as f64 now.
    test()
        .run("[4000000000 4000000001] | math variance")
        .expect_value_eq(0.25)
}

#[test]
fn variance_ints_above_2_pow_53_are_approximate() -> Result {
    // Documented trade-off, not a regression: these are exact as i64 but not as
    // f64, so the result is approximate (the true variance is 0.25) instead of an
    // error. `math avg` already behaves the same way.
    test()
        .run("[9007199254740993 9007199254740994] | math variance")
        .expect_value_eq(2.0)
}
