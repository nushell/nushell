use nu_test_support::prelude::*;

// Starting a path or URL that exists would launch a real application, so only the failure is tested.
#[test]
fn start_reports_a_missing_path() -> Result {
    let err = test()
        .run("start this_file_does_not_exist.txt")
        .expect_shell_error()?;
    assert_contains("Cannot find file or URL", err.generic_error()?);
    Ok(())
}
