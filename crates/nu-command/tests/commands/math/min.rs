use nu_test_support::prelude::*;

#[test]
fn const_min() -> Result {
    test()
        .run("const MIN = [1 3 5] | math min; $MIN")
        .expect_value_eq(1)
}

#[test]
fn cannot_min_infinite_range() -> Result {
    let outcome = test().run("0.. | math min").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
