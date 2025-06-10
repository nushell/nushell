use nu_test_support::nu;

#[test]
fn const_max() {
    let actual = nu!("const MAX = [1 3 5] | math max; $MAX");
    assert_eq!(actual.out, "5");
}

#[test]
fn cannot_max_infinite_range() {
    let actual = nu!("0.. | math max");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
