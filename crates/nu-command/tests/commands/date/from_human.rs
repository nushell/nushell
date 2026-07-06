use nu_test_support::prelude::*;

#[test]
fn from_human_list_of_strings() -> Result {
    let actual: i64 = test().run(
        r#"["Today at 18:30", "Tomorrow at 09:00", "In 5 minutes"] | date from-human | length"#,
    )?;
    assert_eq!(actual, 3);
    Ok(())
}

#[test]
fn from_human_single_string_returns_date() -> Result {
    let code = "'Today at 18:30' | date from-human | describe";
    test().run(code).expect_value_eq("datetime")
}
