use nu_test_support::prelude::*;

#[test]
fn can_round_very_large_numbers() -> Result {
    let outcome: i64 = test().run("18.1372544780074142289927665486772012345 | math round")?;

    assert_eq!(outcome, 18);
    Ok(())
}

#[test]
fn can_round_very_large_numbers_with_precision() -> Result {
    let outcome: f64 = test()
        .run("18.13725447800741422899276654867720121457878988 | math round --precision 10")?;

    assert_eq!(outcome, 18.137254478);
    Ok(())
}

#[test]
fn can_round_integer_with_negative_precision() -> Result {
    let outcome: f64 = test().run("123 | math round --precision -1")?;

    assert_eq!(outcome, 120.0);
    Ok(())
}

#[test]
fn can_round_float_with_negative_precision() -> Result {
    let outcome: f64 = test().run("123.3 | math round --precision -1")?;

    assert_eq!(outcome, 120.0);
    Ok(())
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
    let outcome: i64 = test().run("const ROUND = 18.345 | math round; $ROUND")?;
    assert_eq!(outcome, 18);
    Ok(())
}

#[test]
fn can_round_range_into_list() -> Result {
    let actual: Value = test().run("(1.0)..(1.2)..(2.0) | math round")?;
    let expected: Value = test().run("[1 1 1 2 2 2]")?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn cannot_round_infinite_range() -> Result {
    let outcome = test().run("0.. | math round").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}
