use nu_test_support::prelude::*;

#[test]
fn const_max() -> Result {
    let outcome: i64 = test().run("const MAX = [1 3 5] | math max; $MAX")?;
    assert_eq!(outcome, 5);
    Ok(())
}

#[test]
fn cannot_max_infinite_range() -> Result {
    let outcome = test().run("0.. | math max").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
