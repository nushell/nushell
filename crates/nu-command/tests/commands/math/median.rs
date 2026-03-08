use nu_test_support::prelude::*;

#[test]
fn median_numbers_with_even_rows() -> Result {
    let code = r#"
         echo [10 6 19 21 4]
         | math median
     "#;
    let outcome: i64 = test().run(code)?;

    assert_eq!(outcome, 10);
    Ok(())
}

#[test]
fn median_numbers_with_odd_rows() -> Result {
    let code = r#"
         echo [3 8 9 12 12 15]
         | math median
     "#;
    let outcome: f64 = test().run(code)?;

    assert_eq!(outcome, 10.5);
    Ok(())
}

#[test]
fn median_mixed_numbers() -> Result {
    let code = r#"
         echo [-11.5 -13.5 10]
         | math median
     "#;
    let outcome: f64 = test().run(code)?;

    assert_eq!(outcome, -11.5);
    Ok(())
}

#[test]
fn const_median() -> Result {
    let outcome: i64 = test().run("const MEDIAN = [1 3 5] | math median; $MEDIAN")?;
    assert_eq!(outcome, 3);
    Ok(())
}

#[test]
fn cannot_median_infinite_range() -> Result {
    let outcome = test().run("0.. | math median").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
