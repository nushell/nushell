use nu_test_support::nu;

#[test]
fn const_min() {
    let actual = nu!("const MIN = [1 3 5] | math min; $MIN");
    assert_eq!(actual.out, "1");
}
