use nu_test_support::nu;

#[test]
fn format_duration() {
    let actual = nu!(r#"1MB | format filesize kB"#);

    assert_eq!("1000 kB", actual.out);
}

#[test]
fn format_duration_with_invalid_unit() {
    let actual = nu!(r#"1MB | format filesize sec"#);

    assert!(actual.err.contains("invalid_unit"));
}
