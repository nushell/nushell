use nu_test_support::nu;

#[test]
fn const_log() {
    let actual = nu!("const LOG = 16 | math log 2; $LOG");
    assert_eq!(actual.out, "4.0");
}

#[test]
fn can_log_range_into_list() {
    let actual = nu!("1..5 | math log 2");
    let expected = nu!("[1 2 3 4 5] | math log 2");

    assert_eq!(actual.out, expected.out);
}

#[test]
fn cannot_log_infinite_range() {
    let actual = nu!("1.. | math log 2");

    assert!(actual.err.contains("nu::shell::incorrect_value"));
}
