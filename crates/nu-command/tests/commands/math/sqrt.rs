use nu_test_support::prelude::*;

#[test]
fn can_sqrt_numbers() -> Result {
    test()
        .run("echo [0.25 2 4] | math sqrt | math sum")
        .expect_value_eq(3.914213562373095)
}

#[test]
fn can_sqrt_irrational() -> Result {
    test()
        .run("echo 2 | math sqrt")
        .expect_value_eq(std::f64::consts::SQRT_2)
}

#[test]
fn can_sqrt_perfect_square() -> Result {
    test().run("echo 4 | math sqrt").expect_value_eq(2.0)
}

#[test]
fn const_sqrt() -> Result {
    test()
        .run("const SQRT = 4 | math sqrt; $SQRT")
        .expect_value_eq(2.0)
}

#[test]
fn can_sqrt_range() -> Result {
    let expected: Value = test().run("[0 1 2 3 4 5] | math sqrt")?;
    test().run("0..5 | math sqrt").expect_value_eq(expected)
}

#[test]
fn cannot_sqrt_infinite_range() -> Result {
    let outcome = test().run("0.. | math sqrt").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
