use nu_test_support::prelude::*;

#[test]
fn const_floor() -> Result {
    let outcome: i64 = test().run("const FLOOR = 15.5 | math floor; $FLOOR")?;
    assert_eq!(outcome, 15);
    Ok(())
}

#[test]
fn can_floor_range_into_list() -> Result {
    let actual: Value = test().run("(1.8)..(3.8) | math floor")?;
    let expected: Value = test().run("[1 2 3]")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_floor_infinite_range() -> Result {
    let outcome = test().run("0.. | math floor").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
