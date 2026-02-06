use nu_test_support::nu;

#[test]
fn format_filesize_without_fraction_keeps_old_output() {
    let actual = nu!(r#"1MB | format filesize kB"#);

    assert_eq!("1000 kB", actual.out);
}

#[test]
fn format_filesize_respects_float_precision_for_fractional_values() {
    let actual = nu!(r#"
        $env.config = ($env.config | upsert float_precision 5)
        1024B | format filesize kB
    "#);

    assert_eq!("1.02400 kB", actual.out);
}

#[test]
fn format_filesize_with_invalid_unit() {
    let actual = nu!(r#"1MB | format filesize sec"#);

    assert!(actual.err.contains("invalid_unit"));
}
