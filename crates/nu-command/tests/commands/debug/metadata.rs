use nu_test_support::prelude::*;

/// When pipeline input is collected into null/nothing, `metadata <expr>` should still work.
/// Regression: https://github.com/nushell/nushell/issues/16600
#[test]
fn metadata_positional_with_null_pipeline_input() -> Result {
    let code = r#"null | metadata 42 | get span | describe"#;
    test().run(code).expect_value_eq("record<start: int, end: int>")
}

#[test]
fn metadata_positional_still_errors_with_real_pipeline_input() -> Result {
    let code = r#"1 | metadata 2"#;
    let err = test().run(code).expect_error()?;
    assert!(
        matches!(err, ShellError::IncompatibleParameters { .. }),
        "expected IncompatibleParameters, got {err:?}"
    );
    Ok(())
}
