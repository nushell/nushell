use nu_test_support::prelude::*;

#[test]
fn test_kill_invalid_pid() -> Result {
    let err = test()
        .run_with_data("kill $in", i32::MAX)
        .expect_shell_error()?;

    assert_contains("process didn't terminate successfully", err.to_string());
    Ok(())
}
