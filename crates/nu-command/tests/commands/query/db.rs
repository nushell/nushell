use nu_test_support::nu;

#[cfg(feature = "sqlite")]
#[test]
fn can_query_single_table() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample.db
        | query db "select * from strings"
        | where x =~ ell
        | length
    "#);

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn invalid_sql_fails() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open sample.db
        | query db "select *asdfasdf"
    "#);

    assert!(actual.err.contains("syntax error"));
}

#[cfg(feature = "sqlite")]
#[test]
fn invalid_input_fails() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        "foo" | query db "select * from asdf"
    "#);

    assert!(actual.err.contains("can't convert string"));
}
