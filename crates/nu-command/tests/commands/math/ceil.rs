use nu_test_support::prelude::*;

#[test]
fn const_ceil() -> Result {
    let outcome: i64 = test().run("const CEIL = 1.5 | math ceil; $CEIL")?;
    assert_eq!(outcome, 2);
    Ok(())
}

#[test]
fn can_ceil_range_into_list() -> Result {
    let actual: Value = test().run("(1.8)..(3.8) | math ceil")?;
    let expected: Value = test().run("[2 3 4]")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_ceil_infinite_range() -> Result {
    let outcome = test().run("0.. | math ceil").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
