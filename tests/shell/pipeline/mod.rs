mod commands;

use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn doesnt_break_on_utf8() {
    let actual = nu!(cwd: ".", "echo ö");

    assert_eq!(actual, "ö", "'{}' should contain ö", actual);
}

#[test]
fn visualize_one_table_given_rows_with_same_columns_regardless_of_their_order_per_row() {
    Playground::setup("visualize_table_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "unordered_columns.txt",
            r#"
                [
                    {"name":"Andrés", "rusty_luck": 1         },
                    {"rusty_luck": 1,       "name": "Jonathan"},
                ]
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open unordered_columns.txt
                | from-json
                | echo $it
            "#
        ));

        let name_column_indices: Vec<_> = actual.match_indices("name").collect();

        assert!(
            name_column_indices.len() == 1,
            "Expected one 'name' when displaying the table: {}",
            actual,
        );
    })
}

#[test]
fn visualize_one_table_given_rows_with_regular_values() {
    Playground::setup("visualize_table_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo andres yehuda jonathan jason
                | echo $it
            "#
        ));

        let value_column_indices: Vec<_> = actual.match_indices("<value>").collect();

        assert!(
            value_column_indices.len() == 1,
            "Expected one '<value>' when displaying the table: {}",
            actual,
        );
    });
}
