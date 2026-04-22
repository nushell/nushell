use nu_test_support::prelude::*;

#[test]
fn test_kill_invalid_pid() -> Result {
    let pid = i32::MAX;
    let err = test().run(format!("kill {pid}")).expect_shell_error()?;

    assert_contains("process didn't terminate successfully", err.to_string());
    Ok(())
}
