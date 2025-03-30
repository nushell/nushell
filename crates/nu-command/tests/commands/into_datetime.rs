use nu_test_support::nu;

#[test]
fn into_datetime_from_record() {
    let actual = nu!(
        r#"{year: 2023, month: 1, day: 1, hour: 0, minute: 0, second: 0, millisecond: 0, microsecond: 0, nanosecond: 0, timezone: "+01:00"} | into datetime"#
    );
    let expected = nu!(r#"'01/01/2023' | into datetime"#);

    assert_eq!(expected.out, actual.out);
}

#[test]
fn into_datetime_table_column() {
    let actual = nu!(r#"[[date]; ["2022-01-01"] ["2023-01-01"]] | into datetime date"#);

    assert!(actual.out.contains(" ago"));
}
