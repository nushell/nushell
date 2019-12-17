use nu_test_support::{nu, pipeline};

#[test]
fn table_to_sqlite_and_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | to-sqlite
            | from-sqlite
            | get table_values
            | nth 2
            | get x
            | echo $it
        "#
    ));

    assert_eq!(actual, "hello");
}
