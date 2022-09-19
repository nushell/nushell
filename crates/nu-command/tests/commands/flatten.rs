use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn flatten_nested_tables_with_columns() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[origin, people]; [Ecuador, ('Andres' | wrap name)]]
                 [[origin, people]; [Nu, ('nuno' | wrap name)]]
            | flatten --all | flatten --all
            | get name
            | str join ','
        "#
    ));

    assert_eq!(actual.out, "Andres,nuno");
}

#[test]
fn flatten_nested_tables_that_have_many_columns() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[origin, people]; [Ecuador, (echo [[name, meal]; ['Andres', 'arepa']])]]
            [[origin, people]; [USA, (echo [[name, meal]; ['Katz', 'nurepa']])]]
            | flatten --all | flatten --all
            | get meal
            | str join ','
        "#
    ));

    assert_eq!(actual.out, "arepa,nurepa");
}

#[test]
fn flatten_nested_tables() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[Andrés, Nicolás, Robalino]] | flatten | get 1
        "#
    ));

    assert_eq!(actual.out, "Nicolás");
}

#[test]
fn flatten_row_column_explicitly() {
    Playground::setup("flatten_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.json",
            r#"
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
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.json | flatten people --all | where name == Andres | length"
        );

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn flatten_row_columns_having_same_column_names_flats_separately() {
    Playground::setup("flatten_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.json",
            r#"
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
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.json | flatten --all | flatten people city | get city_name | length"
        );

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn flatten_table_columns_explicitly() {
    Playground::setup("flatten_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.json",
            r#"
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
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.json | flatten city --all | where people.name == Katz | length"
        );

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn flatten_more_than_one_column_that_are_subtables_not_supported() {
    Playground::setup("flatten_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.json",
            r#"
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
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.json | flatten tags city --all"
        );

        assert!(actual.err.contains("tried flattening"));
        assert!(actual.err.contains("but is flattened already"));
    })
}
