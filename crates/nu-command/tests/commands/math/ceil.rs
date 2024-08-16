use nu_test_support::nu;

#[test]
fn const_ceil() {
    let actual = nu!("const CEIL = 1.5 | math ceil; $CEIL");
    assert_eq!(actual.out, "2");
}
