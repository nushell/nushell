use nu_test_support::nu;

#[test]
fn const_abs() {
    let actual = nu!("const ABS = -5.5 | math abs; $ABS");
    assert_eq!(actual.out, "5.5");
}

#[test]
fn cannot_abs_range() {
    let actual = nu!("0..5 | math abs");

    assert!(actual.err.contains("nu::parser::input_type_mismatch"));
}
