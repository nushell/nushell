use nu_test_support::nu;

#[test]
fn const_avg() {
    let actual = nu!("const SDEV = [1 2] | math stddev; $SDEV");
    assert_eq!(actual.out, "0.5");
}

#[test]
fn cannot_stddev_range() {
    let actual = nu!("0..5 | math stddev");

    assert!(actual.err.contains("nu::parser::input_type_mismatch"));
}
