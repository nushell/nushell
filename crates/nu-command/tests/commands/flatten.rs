use pretty_assertions::assert_matches;

use nu_test_support::prelude::*;

#[test]
fn flatten_nested_tables_with_columns() -> Result {
    let code = "
        [
            [[origin, people]; [Ecuador, ('Andres' | wrap name)]]
            [[origin, people]; [Nu, ('nuno' | wrap name)]]
        ]
        | flatten --all
        | flatten --all
        | get name
    ";
    test().run(code).expect_value_eq(["Andres", "nuno"])
}

#[test]
fn flatten_nested_tables_that_have_many_columns() -> Result {
    let code = "
        [
            [
                [origin, people];
                [
                    Ecuador,
                    [[name, meal]; ['Andres', 'arepa']]
                ]
            ]
            [
                [origin, people];
                [
                    USA,
                    [[name, meal]; ['Katz', 'nurepa']]
                ]
            ]
        ]
        | flatten --all
        | flatten --all
        | get meal
    ";
    test().run(code).expect_value_eq(["arepa", "nurepa"])
}

#[test]
fn flatten_nested_tables() -> Result {
    let code = "
        [[Andrés, Nicolás, Robalino]]
        | flatten
        | get 1
    ";
    test().run(code).expect_value_eq("Nicolás")
}

#[test]
fn flatten_row_column_explicitly() -> Result {
    let code = r#"
        [
            {
                "people": {
                    "name": "Andres",
                    "meal": "arepa"
                }
            },
            {
                "people": {
                    "name": "Katz",
                    "meal": "nurepa"
                }
            }
        ]
        | flatten people --all
        | where name == Andres
        | length
    "#;
    test().run(code).expect_value_eq(1)
}

#[test]
fn flatten_row_columns_having_same_column_names_flats_separately() -> Result {
    let code = r#"
        [
            {
                "people": {
                    "name": "Andres",
                    "meal": "arepa"
                },
                "city": [{"name": "Guayaquil"}, {"name": "Samborondón"}]
            },
            {
                "people": {
                    "name": "Katz",
                    "meal": "nurepa"
                },
                "city": [{"name": "Oregon"}, {"name": "Brooklin"}]
            }
        ]
        | flatten --all
        | flatten people city
        | get city_name
        | length
    "#;
    test().run(code).expect_value_eq(4)
}

#[test]
fn flatten_nested_table_renames_conflicting_column_after_flattened_column() -> Result {
    let code = "
        [[b, a]; [[[a]; [9]], 1]]
        | flatten --all b
    ";
    test()
        .run(code)
        .expect_value_eq(test_table![["b_a", "a"]; [9, 1]])
}

#[test]
fn flatten_nested_record_renames_conflicting_column_after_flattened_column() -> Result {
    test()
        .run("{b: {a: 9}, a: 1} | flatten b")
        .expect_value_eq(test_table![["b_a", "a"]; [9, 1]])
}

#[test]
fn flatten_table_columns_explicitly() -> Result {
    let code = r#"
        [
            {
                "people": {
                    "name": "Andres",
                    "meal": "arepa"
                },
                "city": ["Guayaquil", "Samborondón"]
            },
            {
                "people": {
                    "name": "Katz",
                    "meal": "nurepa"
                },
                "city": ["Oregon", "Brooklin"]
            }
        ]
        | flatten city --all
        | where people.name == Katz
        | length
    "#;
    test().run(code).expect_value_eq(2)
}

#[test]
fn flatten_more_than_one_column_that_are_subtables_not_supported() -> Result {
    let code = r#"
        [
            {
                "people": {
                    "name": "Andres",
                    "meal": "arepa"
                }
                "tags": ["carbohydrate", "corn", "maiz"],
                "city": ["Guayaquil", "Samborondón"]
            },
            {
                "people": {
                    "name": "Katz",
                    "meal": "nurepa"
                },
                "tags": ["carbohydrate", "shell food", "amigos flavor"],
                "city": ["Oregon", "Brooklin"]
            }
        ]
        | flatten tags city --all
    "#;
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::UnsupportedInput { msg, .. }
        if msg.contains("tried flattening") && msg.contains("but is flattened already")
    );
    Ok(())
}
