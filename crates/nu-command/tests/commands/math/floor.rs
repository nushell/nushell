use nu_test_support::nu;

#[test]
fn const_floor() {
    let actual = nu!("const FLOOR = 15.5 | math floor; $FLOOR");
    assert_eq!(actual.out, "15");
}

#[test]
fn cannot_floor_range() {
    let actual = nu!("0.. | math floor");

    assert!(actual.err.contains("nu::parser::input_type_mismatch"));
}
