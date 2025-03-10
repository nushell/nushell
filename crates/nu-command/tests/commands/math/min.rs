use nu_test_support::nu;

#[test]
fn const_min() {
    let actual = nu!("const MIN = [1 3 5] | math min; $MIN");
    assert_eq!(actual.out, "1");
}

#[test]
fn cannot_min_infinite_range() {
    let actual = nu!("0.. | math min");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
