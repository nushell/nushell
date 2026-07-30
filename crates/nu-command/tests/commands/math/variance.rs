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
