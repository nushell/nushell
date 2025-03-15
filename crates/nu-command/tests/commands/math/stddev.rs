use nu_test_support::nu;

#[test]
fn const_avg() {
    let actual = nu!("const SDEV = [1 2] | math stddev; $SDEV");
    assert_eq!(actual.out, "0.5");
}

#[test]
fn can_stddev_range() {
    let actual = nu!("0..5 | math stddev");
    let expected = nu!("[0 1 2 3 4 5] | math stddev");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_stddev_infinite_range() {
    let actual = nu!("0.. | math stddev");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
