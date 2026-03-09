use nu_test_support::prelude::*;

#[test]
fn formatter_not_valid() -> Result {
    let code = r#"
        date now | format date '%N'
        "#;

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::TypeMismatch { .. }));
    Ok(())
}

#[test]
fn test_j_q_format_specifiers() -> Result {
    let code = r#"
        "2021-10-22 20:00:12 +01:00" | format date '%J_%Q'
        "#;

    test().run(code).expect_value_eq("20211022_200012")
}

#[test]
fn test_j_q_format_specifiers_current_time() -> Result {
    let code = r#"
        date now | format date '%J_%Q' | str length
        "#;

    // Should be exactly 15 characters: YYYYMMDD_HHMMSS
    test().run(code).expect_value_eq(15)
}

#[test]
fn test_j_format_specifier_date_only() -> Result {
    let code = r#"
        "2021-10-22 20:00:12 +01:00" | format date '%J'
        "#;

    test().run(code).expect_value_eq("20211022")
}

#[test]
fn test_q_format_specifier_time_only() -> Result {
    let code = r#"
        "2021-10-22 20:00:12 +01:00" | format date '%Q'
        "#;

    test().run(code).expect_value_eq("200012")
}

#[test]
fn fails_without_input() -> Result {
    let code = r#"
        format date "%c"
        "#;

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::PipelineEmpty { .. }));
    Ok(())
}

#[test]
fn locale_format_respect_different_locale() -> Result {
    let code = r#"
    "2021-10-22 20:00:12 +01:00" | format date "%c"
    "#;
    let actual: String = test().locale("en_US").run(code)?;
    assert_contains("Fri 22 Oct 2021 08:00:12 PM +01:00", actual);

    let actual: String = test().locale("en_GB").run(code)?;
    assert_contains("Fri 22 Oct 2021 20:00:12 +01:00", actual);

    let actual: String = test().locale("de_DE").run(code)?;
    assert_contains("Fr 22 Okt 2021 20:00:12 +01:00", actual);

    let actual: String = test().locale("zh_CN").run(code)?;
    assert_contains("2021年10月22日 星期五 20时00分12秒", actual);

    let actual: String = test().locale("ja_JP").run(code)?;
    assert_contains("2021年10月22日 20時00分12秒", actual);

    let actual: String = test().locale("fr_FR").run(code)?;
    assert_contains("ven. 22 oct. 2021 20:00:12 +01:00", actual);
    Ok(())
}

#[test]
fn format_respects_locale_from_with_env() -> Result {
    // Tests for https://github.com/nushell/nushell/issues/17321

    let code = r#"
    "2021-10-22 20:00:12 +01:00" | with-env { NU_TEST_LOCALE_OVERRIDE: ko_KR.UTF-8 } { format date "%x %X" }
    "#;
    let actual: String = test().run(code)?;
    assert_contains("2021년 10월 22일 20시 00분 12초", actual);

    let code = r#"
    "2021-10-22 20:00:12 +01:00" | with-env { NU_TEST_LOCALE_OVERRIDE: en_US.UTF-8 } { format date "%x %X" }
    "#;
    let actual: String = test().run(code)?;
    assert_contains("10/22/2021 08:00:12 PM", actual);
    Ok(())
}

#[test]
fn locale_with_different_format_specifiers() -> Result {
    let code = r#"
        "Thu, 26 Oct 2023 22:52:14 +0200" | format date "%x %X"
        "#;
    let actual: String = test().locale("en_US").run(code)?;
    assert_contains("10/26/2023 10:52:14 PM", actual);

    let actual: String = test().locale("nl_NL").run(code)?;
    assert_contains("26-10-23 22:52:14", actual);
    Ok(())
}
