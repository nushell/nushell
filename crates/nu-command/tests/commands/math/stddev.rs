use nu_test_support::prelude::*;

#[test]
fn const_avg() -> Result {
    test()
        .run("const SDEV = [1 2] | math stddev; $SDEV")
        .expect_value_eq(0.5)
}

#[test]
fn can_stddev_range() -> Result {
    let expected: Value = test().run("[0 1 2 3 4 5] | math stddev")?;
    test().run("0..5 | math stddev").expect_value_eq(expected)
}

#[test]
fn cannot_stddev_infinite_range() -> Result {
    let outcome = test().run("0.. | math stddev").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
