use nu_test_support::prelude::*;

#[test]
fn format_filesize_without_fraction_keeps_old_output() -> Result {
    let code = "1MB | format filesize kB";
    test().run(code).expect_value_eq("1000 kB")
}

#[test]
fn format_filesize_respects_float_precision_for_fractional_values() -> Result {
    let code = r#"
        $env.config = ($env.config | upsert float_precision 5)
        1024B | format filesize kB
    "#;

    test().run(code).expect_value_eq("1.02400 kB")
}

#[test]
fn format_filesize_with_invalid_unit() -> Result {
    let code = "1MB | format filesize sec";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit { .. }));
    Ok(())
}
