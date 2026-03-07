use nu_test_support::prelude::*;

#[test]
fn const_avg() -> Result {
    let outcome: Value = test().run("const MODE = [1 3 3 5] | math mode; $MODE")?;
    let expected = Value::test_list(vec![Value::test_int(3)]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn cannot_mode_range() -> Result {
    let outcome = test().run("0..5 | math mode").expect_parse_error()?;

    assert!(matches!(outcome, ParseError::InputMismatch { .. }));
    Ok(())
}
