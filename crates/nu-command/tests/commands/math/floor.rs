use nu_test_support::nu;

#[test]
fn const_floor() {
    let actual = nu!("const FLOOR = 15.5 | math floor; $FLOOR");
    assert_eq!(actual.out, "15");
}

#[test]
fn can_floor_range() {
    let actual = nu!("(1.8)..(1.9)..(2.2) | math floor");
    let expected = nu!("[1 1 2 2 2]");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_floor_infinite_range() {
    let actual = nu!("0.. | math floor");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
