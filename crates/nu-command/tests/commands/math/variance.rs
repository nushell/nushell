use nu_test_support::prelude::*;

#[test]
fn const_variance() -> Result {
    let outcome: f64 = test().run("const VAR = [1 2 3 4 5] | math variance; $VAR")?;
    assert_eq!(outcome, 2.0);
    Ok(())
}

#[test]
fn can_variance_range() -> Result {
    let actual: Value = test().run("0..5 | math variance")?;
    let expected: Value = test().run("[0 1 2 3 4 5] | math variance")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_variance_infinite_range() -> Result {
    let outcome = test().run("0.. | math variance").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
