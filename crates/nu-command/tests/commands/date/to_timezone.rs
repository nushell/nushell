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
