use nu_test_support::nu;

#[test]
fn format_duration() {
    let actual = nu!(r#"1hr | format duration sec"#);

    assert_eq!("3600 sec", actual.out);
}

#[test]
fn format_duration_with_invalid_unit() {
    let actual = nu!(r#"1hr | format duration MB"#);

    assert!(actual.err.contains("invalid_unit"));
}
