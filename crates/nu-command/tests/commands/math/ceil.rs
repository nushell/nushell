use nu_test_support::nu;

#[test]
fn const_ceil() {
    let actual = nu!("const CEIL = 1.5 | math ceil; $CEIL");
    assert_eq!(actual.out, "2");
}

#[test]
fn can_ceil_range_into_list() {
    let actual = nu!("(1.8)..(3.8) | math ceil");
    let expected = nu!("[2 3 4]");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_ceil_infinite_range() {
    let actual = nu!("0.. | math ceil");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
