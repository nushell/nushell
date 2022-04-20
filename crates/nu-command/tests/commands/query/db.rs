use nu_test_support::{nu, pipeline};

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

#[test]
fn invalid_input_fails() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            "foo" | query db "select * from asdf"
        "#
    ));

    assert!(actual.err.contains("pipeline_mismatch"));
}

#[test]
fn scratch() {
    assert!(true);
}
