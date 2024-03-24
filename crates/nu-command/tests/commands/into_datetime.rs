use nu_test_support::nu;

#[test]
fn into_datetime_table_column() {
    let actual = nu!(r#"[[date]; ["2022-01-01"] ["2023-01-01"]] | into datetime date"#);

    assert!(actual.out.contains(" ago"));
}

#[test]
fn invalid_date_caught_panic() {
    let actual = nu!("'2023-13-31' | into datetime");
    assert!(actual
        .err
        .contains("Internal error: Panic when parsing date"));
}
