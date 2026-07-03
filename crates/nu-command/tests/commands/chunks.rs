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
fn chunk_size_not_int_or_filesize() -> Result {
    let err = test()
        .run("[0 1 2] | chunks (echo 1sec)")
        .expect_shell_error()?;
    assert!(matches!(err, ShellError::RuntimeTypeMismatch { .. }));
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

#[test]
fn chunk_binary_with_filesize() -> Result {
    test()
        .run("(0x[11 22 33 44 55 66 77 88] | chunks 3b | length) == 3")
        .expect_value_eq(true)
}

#[test]
fn chunk_binary_with_int() -> Result {
    test()
        .run(
            "0x[11 22 33 44 55 66 77 88] | chunks 3 | each { |chunk| $chunk | length } | to json -r",
        )
        .expect_value_eq("[3,3,2]")
}

#[test]
fn chunk_size_filesize_list_error() -> Result {
    let err = test()
        .run("[1 2 3] | chunks 1kb")
        .expect_shell_error()?;
    assert!(
        matches!(err, ShellError::IncompatibleParametersSingle { .. }),
        "expected IncompatibleParametersSingle, got {err:?}"
    );
    Ok(())
}
