use nu_test_support::prelude::*;

#[test]
fn const_log() -> Result {
    test()
        .run("const LOG = 16 | math log 2; $LOG")
        .expect_value_eq(4.0)
}

#[test]
fn can_log_range_into_list() -> Result {
    let expected: Value = test().run("[1 2 3 4 5] | math log 2")?;
    test().run("1..5 | math log 2").expect_value_eq(expected)
}

#[test]
fn cannot_log_infinite_range() -> Result {
    let outcome = test().run("1.. | math log 2").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn cannot_log_base_one() -> Result {
    let outcome = test().run("2 | math log 1").expect_shell_error()?;

    match outcome {
        ShellError::UnsupportedInput { msg, .. } => {
            assert_eq!(msg, "Base has to be greater than 0 and not equal to 1");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn cannot_log_base_one_float() -> Result {
    let outcome = test().run("2 | math log 1.0").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::UnsupportedInput { .. }));
    Ok(())
}

#[test]
fn cannot_log_base_nan() -> Result {
    // `inf - inf` evaluates to a NaN float, which is an invalid logarithm base.
    let outcome = test()
        .run("2 | math log (inf - inf)")
        .expect_shell_error()?;

    assert!(matches!(outcome, ShellError::UnsupportedInput { .. }));
    Ok(())
}

#[test]
fn can_log_base_two() -> Result {
    test().run("16 | math log 2").expect_value_eq(4.0)
}

#[test]
fn can_log_base_ten() -> Result {
    test().run("100 | math log 10").expect_value_eq(2.0)
}

#[test]
fn can_log_base_e() -> Result {
    // `e` is not a nushell builtin; use its literal value as the base.
    test()
        .run("16 | math log 2.718281828459045")
        .expect_value_eq(2.772588722239781)
}
