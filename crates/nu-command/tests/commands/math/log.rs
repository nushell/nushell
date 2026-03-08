use nu_test_support::prelude::*;

#[test]
fn const_log() -> Result {
    let outcome: f64 = test().run("const LOG = 16 | math log 2; $LOG")?;
    assert_eq!(outcome, 4.0);
    Ok(())
}

#[test]
fn can_log_range_into_list() -> Result {
    let actual: Value = test().run("1..5 | math log 2")?;
    let expected: Value = test().run("[1 2 3 4 5] | math log 2")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_log_infinite_range() -> Result {
    let outcome = test().run("1.. | math log 2").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
