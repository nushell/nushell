use nu_test_support::prelude::*;

#[test]
fn window_size_negative() -> Result {
    let err = test().run("[0 1 2] | window -1").expect_shell_error()?;
    assert!(matches!(err, ShellError::NeedsPositiveValue { .. }));
    Ok(())
}

#[test]
fn window_size_zero() -> Result {
    let err = test().run("[0 1 2] | window 0").expect_shell_error()?;
    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_eq!(msg, "`window_size` cannot be zero");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn window_size_not_int() -> Result {
    let err = test()
        .run("[0 1 2] | window (if true { 1sec })")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::CantConvert { .. }));
    Ok(())
}

#[test]
fn stride_negative() -> Result {
    let err = test()
        .run("[0 1 2] | window 1 -s -1")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::NeedsPositiveValue { .. }));
    Ok(())
}

#[test]
fn stride_zero() -> Result {
    let err = test().run("[0 1 2] | window 1 -s 0").expect_shell_error()?;
    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_eq!(msg, "`stride` cannot be zero");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn stride_not_int() -> Result {
    let err = test()
        .run("[0 1 2] | window 1 -s (if true { 1sec })")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::CantConvert { .. }));
    Ok(())
}

#[test]
fn empty() -> Result {
    test().run("[] | window 2 | is-empty").expect_value_eq(true)
}

#[test]
fn list_stream() -> Result {
    test()
        .run("([0 1 2] | every 1 | window 2) == ([0 1 2] | window 2)")
        .expect_value_eq(true)
}

#[test]
fn table_stream() -> Result {
    test()
        .run(
            "([[foo bar]; [0 1] [2 3] [4 5]] | every 1 | window 2) == ([[foo bar]; [0 1] [2 3] [4 5]] | window 2)",
        )
        .expect_value_eq(true)
}

#[test]
fn no_empty_chunks() -> Result {
    test()
        .run("([0 1 2 3 4 5] | window 3 -s 3 -r | length) == 2")
        .expect_value_eq(true)
}

#[test]
fn same_as_chunks() -> Result {
    test()
        .run("([0 1 2 3 4] | window 2 -s 2 -r) == ([0 1 2 3 4 ] | chunks 2)")
        .expect_value_eq(true)
}

#[test]
fn stride_equal_to_window_size() -> Result {
    test()
        .run("([0 1 2 3] | window 2 -s 2 | flatten) == [0 1 2 3]")
        .expect_value_eq(true)
}

#[test]
fn stride_greater_than_window_size() -> Result {
    test()
        .run("([0 1 2 3 4] | window 2 -s 3 | flatten) == [0 1 3 4]")
        .expect_value_eq(true)
}

#[test]
fn stride_less_than_window_size() -> Result {
    test()
        .run("([0 1 2 3 4 5] | window 3 -s 2 | length) == 2")
        .expect_value_eq(true)
}

#[test]
fn stride_equal_to_window_size_remainder() -> Result {
    test()
        .run("([0 1 2 3 4] | window 2 -s 2 -r | flatten) == [0 1 2 3 4]")
        .expect_value_eq(true)
}

#[test]
fn stride_greater_than_window_size_remainder() -> Result {
    test()
        .run("([0 1 2 3 4] | window 2 -s 3 -r | flatten) == [0 1 3 4]")
        .expect_value_eq(true)
}

#[test]
fn stride_less_than_window_size_remainder() -> Result {
    test()
        .run("([0 1 2 3 4 5] | window 3 -s 2 -r | length) == 3")
        .expect_value_eq(true)
}
