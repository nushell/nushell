use nu_test_support::prelude::*;

#[test]
fn const_product() -> Result {
    test()
        .run("const PROD = [1 3 5] | math product; $PROD")
        .expect_value_eq(15)
}

#[test]
fn cannot_product_infinite_range() -> Result {
    let outcome = test().run("0.. | math product").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
