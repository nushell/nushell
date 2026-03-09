use nu_test_support::prelude::*;

#[test]
fn const_variance() -> Result {
    test()
        .run("const VAR = [1 2 3 4 5] | math variance; $VAR")
        .expect_value_eq(2.0)
}

#[test]
fn can_variance_range() -> Result {
    let expected: Value = test().run("[0 1 2 3 4 5] | math variance")?;
    test().run("0..5 | math variance").expect_value_eq(expected)
}

#[test]
fn cannot_variance_infinite_range() -> Result {
    let outcome = test().run("0.. | math variance").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
