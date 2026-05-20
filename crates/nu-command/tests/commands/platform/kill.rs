use nu_test_support::prelude::*;

#[test]
fn test_kill_invalid_pid() -> Result {
    let err = test()
        .run_with_data("kill $in", i32::MAX)
        .expect_shell_error()?;

    assert_contains("process didn't terminate successfully", err.to_string());
    Ok(())
}

#[cfg(unix)]
#[test]
fn kill_negative_signal_shorthand_requires_explicit_signal_flag() -> Result {
    let err = test().run("kill -9").expect_shell_error()?;

    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_contains("kill -s 9 <pid>", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[cfg(unix)]
#[test]
fn kill_negative_zero_signal_shorthand_requires_explicit_signal_flag() -> Result {
    let err = test().run("kill -0").expect_shell_error()?;

    match err {
        ShellError::IncorrectValue { msg, .. } => {
            assert_contains("kill -s 0 <pid>", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}
