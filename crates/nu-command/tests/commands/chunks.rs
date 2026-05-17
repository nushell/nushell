use nu_test_support::prelude::*;

#[test]
fn chunk_size_negative() -> Result {
    let err = test().run("[0 1 2] | chunks -1").expect_shell_error()?;
    assert!(matches!(err, ShellError::NeedsPositiveValue { .. }));
    Ok(())
}

#[test]
fn chunk_size_zero() -> Result {
    let err = test().run("[0 1 2] | chunks 0").expect_shell_error()?;
    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_eq!(msg, "`chunk_size` cannot be zero");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn chunk_size_not_int() -> Result {
    let err = test()
        .run("[0 1 2] | chunks (if true { 1sec })")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::CantConvert { .. }));
    Ok(())
}

#[test]
fn empty() -> Result {
    test().run("[] | chunks 2 | is-empty").expect_value_eq(true)
}

#[test]
fn list_stream() -> Result {
    test()
        .run("([0 1 2] | every 1 | chunks 2) == ([0 1 2] | chunks 2)")
        .expect_value_eq(true)
}

#[test]
fn table_stream() -> Result {
    test()
        .run(
            "([[foo bar]; [0 1] [2 3] [4 5]] | every 1 | chunks 2) == ([[foo bar]; [0 1] [2 3] [4 5]] | chunks 2)",
        )
        .expect_value_eq(true)
}

#[test]
fn no_empty_chunks() -> Result {
    test()
        .run("([0 1 2 3 4 5] | chunks 3 | length) == 2")
        .expect_value_eq(true)
}
