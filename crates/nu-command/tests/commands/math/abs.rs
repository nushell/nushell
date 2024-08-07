use nu_test_support::nu;

#[test]
fn const_abs() {
    let actual = nu!("const ABS = -5.5 | math abs; $ABS");
    assert_eq!(actual.out, "5.5");
}
