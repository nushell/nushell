use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
fn to_timezone_list_of_strings() -> Result {
    let actual: i64 = test().run(
        r#"["2020-10-10 10:00:00 +02:00", "2020-10-10 12:00:00 +02:00"] | date to-timezone "+0500" | length"#,
    )?;
    assert_eq!(actual, 2);
    Ok(())
}

#[test]
fn to_timezone_list_of_datetimes() -> Result {
    let actual: i64 = test().run(
        "[2021-10-22T10:00:00+02:00, 2021-10-23T10:00:00+02:00] | date to-timezone \"+0500\" | length",
    )?;
    assert_eq!(actual, 2);
    Ok(())
}

#[test]
fn to_timezone_single_date_string() -> Result {
    let code =
        r#""2020-10-10 10:00:00 +02:00" | date to-timezone "+0500" | into record | get timezone"#;
    test().run(code).expect_value_eq("+05:00")
}

#[rstest]
#[case::hours_only("+05", "+05:00:00")]
#[case::hours_whitespace("+05 \t\n", "+05:00:00")]
#[case::compact_minutes("+0530", "+05:30:00")]
#[case::spaced_minutes("+05 30", "+05:30:00")]
#[case::spaced_colon("+05 : 30", "+05:30:00")]
#[case::unicode_whitespace("+05\u{2003}:\u{2003}30", "+05:30:00")]
#[case::spaced_colon_with_seconds("-00 : 44:30", "-00:44:30")]
#[case::negative_offset("-05:30", "-05:30:00")]
#[case::trailing_whitespace("+05:30 \t\n", "+05:30:00")]
#[case::zero_seconds("+05:30:00", "+05:30:00")]
#[case::seconds("+05:30:15", "+05:30:15")]
#[case::negative_seconds("-00:44:30", "-00:44:30")]
#[case::seconds_only("-00:00:59", "-00:00:59")]
#[case::seconds_whitespace("+05:30:15 \t\n", "+05:30:15")]
fn to_timezone_accepts_numeric_offsets(#[case] zone: &str, #[case] offset: &str) -> Result {
    test()
        .run_with_data(
            r#"let zone = $in; 2026-01-01T12:00:00Z | date to-timezone $zone | format date "%::z""#,
            zone,
        )
        .expect_value_eq(offset)
}

#[rstest]
#[case::letters("+05:30XYZ")]
#[case::extra_digit("+05301")]
#[case::short_seconds("+05:30:1")]
#[case::out_of_range_seconds("+05:30:60")]
#[case::seconds_suffix("+05:30:15X")]
#[case::non_ascii_seconds("+05:30:é")]
#[case::non_ascii_seconds_suffix("+05:30:15é")]
#[case::trailing_colon("+05:30:")]
#[case::non_ascii("-05:30é")]
#[case::missing_minutes("+05:")]
#[case::repeated_colon_without_minutes("+05::")]
#[case::missing_minutes_with_whitespace("+05: \t\n")]
#[case::repeated_colons("+05:::30")]
#[case::separated_colons("+05 : : 30")]
#[case::missing_minutes_with_unicode_whitespace("+05\u{2003}:\u{2003}")]
fn to_timezone_rejects_invalid_numeric_offsets(#[case] zone: &str) -> Result {
    let err = test()
        .run_with_data(
            "let zone = $in; 2026-01-01T12:00:00Z | date to-timezone $zone | ignore",
            zone,
        )
        .expect_shell_error()?;
    assert!(
        matches!(err, ShellError::TypeMismatch { err_message, .. } if err_message == "invalid time zone")
    );
    Ok(())
}

#[rstest]
#[case::lowercase_utc("utc", "+0000")]
#[case::lowercase_area_location("america/new_york", "-0500")]
#[case::lowercase_alias("us/pacific", "-0800")]
#[case::mixed_case("eUROPE/lONDON", "+0000")]
#[case::trailing_newline("America/New_York\n", "-0500")]
#[case::surrounding_spaces(" europe/london ", "+0000")]
fn to_timezone_name_ignores_case_and_whitespace(
    #[case] zone: &str,
    #[case] offset: &str,
) -> Result {
    test()
        .run_with_data(
            r#"let zone = $in; 2026-01-01T12:00:00Z | date to-timezone $zone | format date "%z""#,
            zone,
        )
        .expect_value_eq(offset)
}
