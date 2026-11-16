use nu_test_support::prelude::*;

#[test]
fn convert_into_float_invalid_string_error_types() -> Result {
    let err = test().run("'invalid' | into float").expect_shell_error()?;
    match err {
        ShellError::CantConvert {
            to_type,
            from_type,
            help,
            span,
        } => {
            assert_eq!(to_type, "float");
            assert_eq!(from_type, "string");
            assert_eq!(help.as_deref(), Some("invalid float literal"));
            assert_eq!(span.end - span.start, "'invalid'".len());
        }
        err => return Err(err.into()),
    }

    Ok(())
}
