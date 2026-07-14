use nu_test_support::prelude::*;

#[test]
fn humanize_list_of_date_strings_returns_list() -> Result {
    let actual: i64 = test().run(
        r#"["2021-10-22 20:00:12 +01:00", "2021-10-23 20:00:00 +01:00"] | date humanize | length"#,
    )?;
    assert_eq!(actual, 2);
    Ok(())
}

#[test]
fn humanize_list_of_datetimes_returns_list() -> Result {
    let actual: i64 = test()
        .run("[2021-10-22T20:00:12+01:00, 2021-10-23T20:00:00+01:00] | date humanize | length")?;
    assert_eq!(actual, 2);
    Ok(())
}

#[test]
fn humanize_single_date_string_returns_string() -> Result {
    let code = r#""2021-10-22 20:00:12 +01:00" | date humanize | describe"#;
    test().run(code).expect_value_eq("string")
}
