use nu_test_support::nu;

#[test]
fn fails_on_datetime_input() {
    let actual = nu!("seq date --begin-date (date now)");

    assert!(actual.err.contains("Type mismatch"))
}

#[test]
fn fails_when_increment_not_integer_or_duration() {
    let actual = nu!("seq date --begin-date 2020-01-01 --increment 1.1");

    assert!(actual
        .err
        .contains("expected one of a list of accepted shapes: [Duration, Int]"))
}
