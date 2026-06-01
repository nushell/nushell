use nu_test_support::prelude::*;

#[test]
fn const_abs() -> Result {
    let code = "const ABS = -5.5 | math abs; $ABS";
    test().run(code).expect_value_eq(5.5)
}

#[test]
fn can_abs_range_into_list() -> Result {
    let expected: String = test().run("1.5..10.5 | to text")?;
    test()
        .run("-1.5..-10.5 | math abs | to text")
        .expect_value_eq(expected)
}

#[test]
fn cannot_abs_infinite_range() -> Result {
    let outcome = test().run("0.. | math abs").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[test]
fn abs_int_min_overflows_without_panic() -> Result {
    // i64::MIN has no representable absolute value; `math abs` must report an
    // overflow error rather than panic. i64::MIN is produced via `into int` to
    // avoid relying on negated-literal parsing.
    let code = "0x[00 00 00 00 00 00 00 80] | into int --endian little | math abs";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::OperatorOverflow { .. }));
    Ok(())
}

#[test]
fn abs_duration_min_overflows_without_panic() -> Result {
    let code = "0x[00 00 00 00 00 00 00 80] | into int --endian little | into duration | math abs";
    let outcome = test().run(code).expect_shell_error()?;

    assert!(matches!(outcome, ShellError::OperatorOverflow { .. }));
    Ok(())
}
