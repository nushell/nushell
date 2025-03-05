use nu_test_support::nu;

#[test]
fn const_log() {
    let actual = nu!("const LOG = 16 | math log 2; $LOG");
    assert_eq!(actual.out, "4");
}

#[test]
fn cannot_log_range() {
    let actual = nu!("0.. | math log 2");

    assert!(actual.err.contains("nu::parser::input_type_mismatch"));
}
