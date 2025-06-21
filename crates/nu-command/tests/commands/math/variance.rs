use nu_test_support::nu;

#[test]
fn const_variance() {
    let actual = nu!("const VAR = [1 2 3 4 5] | math variance; $VAR");
    assert_eq!(actual.out, "2.0");
}

#[test]
fn can_variance_range() {
    let actual = nu!("0..5 | math variance");
    let expected = nu!("[0 1 2 3 4 5] | math variance");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_variance_infinite_range() {
    let actual = nu!("0.. | math variance");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
