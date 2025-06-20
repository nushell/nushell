use nu_test_support::nu;

#[test]
fn can_round_very_large_numbers() {
    let actual = nu!("18.1372544780074142289927665486772012345 | math round");

    assert_eq!(actual.out, "18")
}

#[test]
fn can_round_very_large_numbers_with_precision() {
    let actual = nu!("18.13725447800741422899276654867720121457878988 | math round --precision 10");

    assert_eq!(actual.out, "18.137254478")
}

#[test]
fn can_round_integer_with_negative_precision() {
    let actual = nu!("123 | math round --precision -1");

    assert_eq!(actual.out, "120.0")
}

#[test]
fn can_round_float_with_negative_precision() {
    let actual = nu!("123.3 | math round --precision -1");

    assert_eq!(actual.out, "120.0")
}

#[test]
fn fails_with_wrong_input_type() {
    let actual = nu!("\"not_a_number\" | math round");

    assert!(actual.err.contains("command doesn't support"))
}

#[test]
fn const_round() {
    let actual = nu!("const ROUND = 18.345 | math round; $ROUND");
    assert_eq!(actual.out, "18");
}

#[test]
fn can_round_range_into_list() {
    let actual = nu!("(1.0)..(1.2)..(2.0) | math round");
    let expected = nu!("[1 1 1 2 2 2]");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_round_infinite_range() {
    let actual = nu!("0.. | math round");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
