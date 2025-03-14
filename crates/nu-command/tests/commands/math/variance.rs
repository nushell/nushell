use nu_test_support::nu;

#[test]
fn const_variance() {
    let actual = nu!("const VAR = [1 2 3 4 5] | math variance; $VAR");
    assert_eq!(actual.out, "2");
}

#[test]
fn cannot_variance_range() {
    let actual = nu!("0..5 | math variance");

    assert!(actual.err.contains("nu::parser::input_type_mismatch"));
}
