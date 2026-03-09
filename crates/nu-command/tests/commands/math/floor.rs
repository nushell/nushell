use nu_test_support::prelude::*;

#[test]
fn const_floor() -> Result {
    test()
        .run("const FLOOR = 15.5 | math floor; $FLOOR")
        .expect_value_eq(15)
}

#[test]
fn can_floor_range_into_list() -> Result {
    let expected: Value = test().run("[1 2 3]")?;
    test()
        .run("(1.8)..(3.8) | math floor")
        .expect_value_eq(expected)
}

#[test]
fn cannot_floor_infinite_range() -> Result {
    let outcome = test().run("0.. | math floor").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
