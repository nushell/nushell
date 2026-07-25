use nu_test_support::prelude::*;

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
