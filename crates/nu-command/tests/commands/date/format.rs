use nu_test_support::{nu, pipeline};

#[test]
fn formatter_not_valid() {
    let actual = nu!(r#"
        date now | format date '%N'
        "#);

    assert!(actual.err.contains("invalid format"));
}

#[test]
fn test_j_q_format_specifiers() {
    let actual = nu!(r#"
        "2021-10-22 20:00:12 +01:00" | format date '%J_%Q'
        "#);

    assert_eq!(actual.out, "20211022_200012");
}

#[test]
fn test_j_q_format_specifiers_current_time() {
    let actual = nu!(r#"
        date now | format date '%J_%Q' | str length
        "#);

    // Should be exactly 15 characters: YYYYMMDD_HHMMSS
    assert_eq!(actual.out, "15");
}

#[test]
fn test_j_format_specifier_date_only() {
    let actual = nu!(r#"
        "2021-10-22 20:00:12 +01:00" | format date '%J'
        "#);

    assert_eq!(actual.out, "20211022");
}

#[test]
fn test_q_format_specifier_time_only() {
    let actual = nu!(r#"
        "2021-10-22 20:00:12 +01:00" | format date '%Q'
        "#);

    assert_eq!(actual.out, "200012");
}

#[test]
fn fails_without_input() {
    let actual = nu!(r#"
        format date "%c"
        "#);

    assert!(actual.err.contains("Pipeline empty"));
}

#[test]
fn locale_format_respect_different_locale() {
    let actual = nu!(
        locale: "en_US",
        pipeline(
            r#"
            "2021-10-22 20:00:12 +01:00" | format date "%c"
            "#
        )
    );
    assert!(actual.out.contains("Fri 22 Oct 2021 08:00:12 PM +01:00"));

    let actual = nu!(
        locale: "en_GB",
        pipeline(
            r#"
            "2021-10-22 20:00:12 +01:00" | format date "%c"
            "#
        )
    );
    assert!(actual.out.contains("Fri 22 Oct 2021 20:00:12 +01:00"));

    let actual = nu!(
        locale: "de_DE",
        pipeline(
            r#"
            "2021-10-22 20:00:12 +01:00" | format date "%c"
            "#
        )
    );
    assert!(actual.out.contains("Fr 22 Okt 2021 20:00:12 +01:00"));

    let actual = nu!(
        locale: "zh_CN",
        pipeline(
            r#"
            "2021-10-22 20:00:12 +01:00" | format date "%c"
            "#
        )
    );
    assert!(actual.out.contains("2021年10月22日 星期五 20时00分12秒"));

    let actual = nu!(
        locale: "ja_JP",
        pipeline(
            r#"
            "2021-10-22 20:00:12 +01:00" | format date "%c"
            "#
        )
    );
    assert!(actual.out.contains("2021年10月22日 20時00分12秒"));

    let actual = nu!(
        locale: "fr_FR",
        pipeline(
            r#"
            "2021-10-22 20:00:12 +01:00" | format date "%c"
            "#
        )
    );
    assert!(actual.out.contains("ven. 22 oct. 2021 20:00:12 +01:00"));
}

#[test]
fn locale_with_different_format_specifiers() {
    let actual = nu!(
    locale: "en_US",
    pipeline(
        r#"
            "Thu, 26 Oct 2023 22:52:14 +0200" | format date "%x %X"
            "#
        )
    );
    assert!(actual.out.contains("10/26/2023 10:52:14 PM"));

    let actual = nu!(
    locale: "nl_NL",
    pipeline(
        r#"
            "Thu, 26 Oct 2023 22:52:14 +0200" | format date "%x %X"
            "#
        )
    );
    assert!(actual.out.contains("26-10-23 22:52:14"));
}
