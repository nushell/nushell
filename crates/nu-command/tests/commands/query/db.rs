use nu_test_support::{nu, pipeline};

#[cfg(feature = "database")]
#[test]
fn can_query_single_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | into db
            | query "select * from strings"
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
            | into db
            | query "select *asdfasdf"
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
            "foo" | into db | query "select * from asdf"
        "#
    ));

    assert!(actual.err.contains("can't convert string"));
}
