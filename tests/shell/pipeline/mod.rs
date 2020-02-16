mod commands;

use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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
            cwd: dirs.test(), "open unordered_columns.txt | from-json"
        );

        let name_column_indices: Vec<_> = actual.match_indices("name").collect();
        let rusty_luck_column_indices: Vec<_> = actual.match_indices("rusty_luck").collect();

        for (index, (name_index, _)) in name_column_indices.iter().enumerate() {
            let (rusty_luck_index, _) = rusty_luck_column_indices[index];

            assert!(name_index < &rusty_luck_index);
        }
    })
}
