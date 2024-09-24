use nu_test_support::nu;

#[test]
fn const_floor() {
    let actual = nu!("const FLOOR = 15.5 | math floor; $FLOOR");
    assert_eq!(actual.out, "15");
}
