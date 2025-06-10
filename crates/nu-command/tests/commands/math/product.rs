use nu_test_support::nu;

#[test]
fn const_product() {
    let actual = nu!("const PROD = [1 3 5] | math product; $PROD");
    assert_eq!(actual.out, "15");
}

#[test]
fn cannot_product_infinite_range() {
    let actual = nu!("0.. | math product");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
