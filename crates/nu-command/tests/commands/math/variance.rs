use nu_test_support::nu;

#[test]
fn const_variance() {
    let actual = nu!("const VAR = [1 2 3 4 5] | math variance; $VAR");
    assert_eq!(actual.out, "2");
}
