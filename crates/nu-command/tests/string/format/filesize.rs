use nu_test_support::prelude::*;

#[test]
fn format_filesize_without_fraction_keeps_old_output() -> Result {
    let code = "1MB | format filesize kB";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1000 kB");
    Ok(())
}

#[test]
fn format_filesize_respects_float_precision_for_fractional_values() -> Result {
    let code = r#"
        $env.config = ($env.config | upsert float_precision 5)
        1024B | format filesize kB
    "#;
    
    let outcome: String = test().run(code)?;
    assert_eq!("1.02400 kB", outcome);
    Ok(())
}

#[test]
fn format_filesize_with_invalid_unit() -> Result {
    let code = "1MB | format filesize sec";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit {..}));
    Ok(())
}
