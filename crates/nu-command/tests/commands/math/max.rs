use nu_test_support::prelude::*;

#[test]
fn const_max() -> Result {
    test()
        .run("const MAX = [1 3 5] | math max; $MAX")
        .expect_value_eq(5)
}

#[test]
fn cannot_max_infinite_range() -> Result {
    let outcome = test().run("0.. | math max").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
