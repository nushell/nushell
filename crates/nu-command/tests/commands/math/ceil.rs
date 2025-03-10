use nu_test_support::nu;

#[test]
fn const_ceil() {
    let actual = nu!("const CEIL = 1.5 | math ceil; $CEIL");
    assert_eq!(actual.out, "2");
}

#[test]
fn cannot_ceil_range() {
    let actual = nu!("0..5 | math ceil");

    assert!(actual.err.contains("nu::parser::input_type_mismatch"));
}
