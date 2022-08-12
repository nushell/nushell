use nu_test_support::{nu, pipeline};

#[cfg(feature = "database")]
#[test]
fn can_query_single_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | query db "select * from strings"
            | where x =~ ell
            | length
        "#
    ));

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "database")]
#[test]
fn invalid_sql_fails() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | query db "select *asdfasdf"
        "#
    ));

    assert!(actual.err.contains("syntax error"));
}

#[cfg(feature = "database")]
#[test]
fn invalid_input_fails() {
    let actual = nu!(
    cwd: "tests/fixtures/formats", pipeline(
        r#"
            "foo" | query db "select * from asdf"
        "#
    ));

    assert!(actual.err.contains("can't convert string"));
}
