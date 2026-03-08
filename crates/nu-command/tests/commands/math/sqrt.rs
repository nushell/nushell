use nu_test_support::prelude::*;

#[test]
fn can_sqrt_numbers() -> Result {
    let outcome: f64 = test().run("echo [0.25 2 4] | math sqrt | math sum")?;

    assert_eq!(outcome, 3.914213562373095);
    Ok(())
}

#[test]
fn can_sqrt_irrational() -> Result {
    let outcome: f64 = test().run("echo 2 | math sqrt")?;

    assert_eq!(outcome, std::f64::consts::SQRT_2);
    Ok(())
}

#[test]
fn can_sqrt_perfect_square() -> Result {
    let outcome: f64 = test().run("echo 4 | math sqrt")?;

    assert_eq!(outcome, 2.0);
    Ok(())
}

#[test]
fn const_sqrt() -> Result {
    let outcome: f64 = test().run("const SQRT = 4 | math sqrt; $SQRT")?;
    assert_eq!(outcome, 2.0);
    Ok(())
}

#[test]
fn can_sqrt_range() -> Result {
    let actual: Value = test().run("0..5 | math sqrt")?;
    let expected: Value = test().run("[0 1 2 3 4 5] | math sqrt")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_sqrt_infinite_range() -> Result {
    let outcome = test().run("0.. | math sqrt").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
