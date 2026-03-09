use nu_test_support::prelude::*;

#[test]
fn can_round_very_large_numbers() -> Result {
    test()
        .run("18.1372544780074142289927665486772012345 | math round")
        .expect_value_eq(18)
}

#[test]
fn can_round_very_large_numbers_with_precision() -> Result {
    test()
        .run("18.13725447800741422899276654867720121457878988 | math round --precision 10")
        .expect_value_eq(18.137254478)
}

#[test]
fn can_round_integer_with_negative_precision() -> Result {
    test()
        .run("123 | math round --precision -1")
        .expect_value_eq(120.0)
}

#[test]
fn can_round_float_with_negative_precision() -> Result {
    test()
        .run("123.3 | math round --precision -1")
        .expect_value_eq(120.0)
}

#[test]
fn fails_with_wrong_input_type() -> Result {
    let outcome = test()
        .run("\"not_a_number\" | math round")
        .expect_parse_error()?;

    assert!(matches!(outcome, ParseError::InputMismatch { .. }));
    Ok(())
}

#[test]
fn const_round() -> Result {
    test()
        .run("const ROUND = 18.345 | math round; $ROUND")
        .expect_value_eq(18)
}

#[test]
fn can_round_range_into_list() -> Result {
    let expected: Value = test().run("[1 1 1 2 2 2]")?;
    test()
        .run("(1.0)..(1.2)..(2.0) | math round")
        .expect_value_eq(expected)
}

#[test]
fn cannot_round_infinite_range() -> Result {
    let outcome = test().run("0.. | math round").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
