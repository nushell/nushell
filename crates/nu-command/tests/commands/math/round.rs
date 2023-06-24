use nu_test_support::nu;

#[test]
fn can_round_very_large_numbers() {
    let actual = nu!("18.1372544780074142289927665486772012345 | math round");

    assert_eq!(actual.out, "18")
}

#[test]
fn can_round_very_large_numbers_with_precision() {
    let actual = nu!("18.13725447800741422899276654867720121457878988 | math round -p 10");

    assert_eq!(actual.out, "18.137254478")
}

#[test]
fn can_round_integer_with_negative_precision() {
    let actual = nu!("123 | math round -p -1");

    assert_eq!(actual.out, "120")
}

#[test]
fn can_round_float_with_negative_precision() {
    let actual = nu!("123.3 | math round -p -1");

    assert_eq!(actual.out, "120")
}

#[test]
fn fails_with_wrong_input_type() {
    let actual = nu!("\"not_a_number\" | math round");

    assert!(actual.err.contains("Input type not supported"))
}
