use nu_test_support::prelude::*;

#[test]
fn const_abs() -> Result {
    let code = "const ABS = -5.5 | math abs; $ABS";
    let outcome: f64 = test().run(code)?;
    assert_eq!(outcome, 5.5);
    Ok(())
}

#[test]
fn can_abs_range_into_list() -> Result {
    let actual: String = test().run("-1.5..-10.5 | math abs | to text")?;
    let expected: String = test().run("1.5..10.5 | to text")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_abs_infinite_range() -> Result {
    let outcome = test().run("0.. | math abs").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
