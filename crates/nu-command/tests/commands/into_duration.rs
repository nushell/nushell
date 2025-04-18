use nu_test_support::nu;

#[test]
fn into_duration_table_column() {
    let actual =
        nu!(r#"[[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value"#);
    let expected = nu!(r#"[[value]; [1sec] [2min] [3hr] [4day] [5wk]]"#);

    assert_eq!(actual.out, expected.out);
}
