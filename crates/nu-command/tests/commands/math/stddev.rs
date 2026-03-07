use nu_test_support::prelude::*;

#[test]
fn const_avg() -> Result {
    let outcome: f64 = test().run("const SDEV = [1 2] | math stddev; $SDEV")?;
    assert_eq!(outcome, 0.5);
    Ok(())
}

#[test]
fn can_stddev_range() -> Result {
    let actual: Value = test().run("0..5 | math stddev")?;
    let expected: Value = test().run("[0 1 2 3 4 5] | math stddev")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_stddev_infinite_range() -> Result {
    let outcome = test().run("0.. | math stddev").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
