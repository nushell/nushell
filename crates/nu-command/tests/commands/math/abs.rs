use nu_test_support::nu;

#[test]
fn const_abs() {
    let actual = nu!("const ABS = -5.5 | math abs; $ABS");
    assert_eq!(actual.out, "5.5");
}

#[test]
fn can_abs_range_into_list() {
    let actual = nu!("-1.5..-10.5 | math abs");
    let expected = nu!("1.5..10.5");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_abs_infinite_range() {
    let actual = nu!("0.. | math abs");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
