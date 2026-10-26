use nu_test_support::prelude::*;

#[test]
fn convert_into_float_success() -> Result {
    test()
        .run("'123.45' | into float")
        .expect_value_eq(123.45)?;
    test().run("123 | into float").expect_value_eq(123.0)?;
    test().run("true | into float").expect_value_eq(1.0)
}

#[test]
fn convert_into_float_invalid_string_error_types() -> Result {
    let err = test().run("'invalid' | into float").expect_shell_error()?;
    match err {
        ShellError::CantConvert {
            to_type,
            from_type,
            help,
            ..
        } => {
            assert_eq!(to_type, "float");
            assert_eq!(from_type, "string");
            assert!(help.is_some());
        }
        err => return Err(err.into()),
    }

    Ok(())
}
