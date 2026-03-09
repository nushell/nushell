use nu_test_support::prelude::*;

#[test]
fn format_duration() -> Result {
    let code = "1hr | format duration sec";
    test().run(code).expect_value_eq("3600 sec")
}

#[test]
fn format_duration_with_invalid_unit() -> Result {
    let code = "1hr | format duration MB";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit { .. }));
    Ok(())
}
