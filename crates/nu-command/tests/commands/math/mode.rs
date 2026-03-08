use nu_test_support::prelude::*;

#[test]
fn const_avg() -> Result {
    let expected = Value::test_list(vec![Value::test_int(3)]);
    test()
        .run("const MODE = [1 3 3 5] | math mode; $MODE")
        .expect_value_eq(expected)
}

#[test]
fn cannot_mode_range() -> Result {
    let outcome = test().run("0..5 | math mode").expect_parse_error()?;

    assert!(matches!(outcome, ParseError::InputMismatch { .. }));
    Ok(())
}
