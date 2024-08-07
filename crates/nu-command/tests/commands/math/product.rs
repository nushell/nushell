use nu_test_support::nu;

#[test]
fn const_product(){
    let actual = nu!("const PROD = [1 3 5] | math product; $PROD");
    assert_eq!(actual.out, "15");
}