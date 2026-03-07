use nu_test_support::prelude::*;

#[test]
fn format_duration() -> Result {
    let code = "1hr | format duration sec";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "3600 sec");
    Ok(())
}

#[test]
fn format_duration_with_invalid_unit() -> Result {
    let code = "1hr | format duration MB";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit {..}));
    Ok(())
}
