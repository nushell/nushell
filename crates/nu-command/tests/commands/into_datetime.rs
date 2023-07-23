use nu_test_support::nu;

#[test]
fn into_datetime_table_column() {
    let actual = nu!(r#"[[date]; ["2022-01-01"] ["2023-01-01"]] | into datetime date"#);

    assert!(actual.out.contains("Sat, 01 Jan 2022"));
    assert!(actual.out.contains("Sun, 01 Jan 2023"));
}
