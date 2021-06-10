#[cfg(feature = "sqlite")]
use nu_test_support::{nu, pipeline};

#[cfg(feature = "sqlite")]
#[test]
fn table_to_sqlite_and_back_into_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | to sqlite
            | from sqlite
            | get table_values
            | nth 2
            | get x
        "#
    ));

    assert_eq!(actual.out, "hello");
}

#[cfg(feature = "sqlite")]
#[test]
fn table_to_sqlite_and_back_into_table_select_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | to sqlite
            | from sqlite -t [strings]
            | get table_names
        "#
    ));

    assert_eq!(actual.out, "strings");
}
