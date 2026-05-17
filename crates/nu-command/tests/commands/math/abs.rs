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
