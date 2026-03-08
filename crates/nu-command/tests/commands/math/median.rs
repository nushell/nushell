use nu_test_support::prelude::*;

#[test]
fn median_numbers_with_even_rows() -> Result {
    let code = r#"
         echo [10 6 19 21 4]
         | math median
     "#;
    test().run(code).expect_value_eq(10)
}

#[test]
fn median_numbers_with_odd_rows() -> Result {
    let code = r#"
         echo [3 8 9 12 12 15]
         | math median
     "#;
    test().run(code).expect_value_eq(10.5)
}

#[test]
fn median_mixed_numbers() -> Result {
    let code = r#"
         echo [-11.5 -13.5 10]
         | math median
     "#;
    test().run(code).expect_value_eq(-11.5)
}

#[test]
fn const_median() -> Result {
    test()
        .run("const MEDIAN = [1 3 5] | math median; $MEDIAN")
        .expect_value_eq(3)
}

#[test]
fn cannot_median_infinite_range() -> Result {
    let outcome = test().run("0.. | math median").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
