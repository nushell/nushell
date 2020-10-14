use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn flatten_nested_tables_with_columns() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[origin, people]; [Ecuador, $(= 'Andres' | wrap name)]]
                 [[origin, people]; [Nu, $(= 'nuno' | wrap name)]]
            | flatten
            | get name
            | str collect ','
        "#
    ));

    assert_eq!(actual.out, "Andres,nuno");
}

#[test]
fn flatten_nested_tables_that_have_many_columns() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[origin, people]; [Ecuador, $(echo [[name, meal]; ['Andres', 'arepa']])]]
                 [[origin, people]; [USA, $(echo [[name, meal]; ['Katz', 'nurepa']])]] 
            | flatten
            | get meal
            | str collect ','
        "#
    ));

    assert_eq!(actual.out, "arepa,nurepa");
}

#[test]
fn flatten_nested_tables() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo [[Andrés, Nicolás, Robalino]] | flatten | nth 1
        "#
    ));

    assert_eq!(actual.out, "Nicolás");
}

#[test]
fn flatten_row_column_explictly() {
    Playground::setup("flatten_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.json",
            r#"
                [
                    {
                        "origin": "Ecuador",
                        "people": {
                            "name": "Andres",
                            "meal": "arepa"
                        },
                        "code": { "id": 1, "references": 2},
                        "tags": ["carbohydrate", "corn", "maiz"],
                        "city": ["Guayaquil", "Samborondón"]
                    },
                    {
                        "origin": "USA",
                        "people": {
                            "name": "Katz",
                            "meal": "nurepa"
                        },
                        "code": { "id": 2, "references": 1},
                        "tags": ["carbohydrate", "shell food", "amigos flavor"],
                        "city": ["Oregon", "Brooklin"]
                    }
                ]
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.json | flatten people | where name == Andres | count"
        );

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn flatten_table_columns_explictly() {
    Playground::setup("flatten_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.json",
            r#"
                [
                    {
                        "origin": "Ecuador",
                        "people": {
                            "name": "Andres",
                            "meal": "arepa"
                        },
                        "code": { "id": 1, "references": 2},
                        "tags": ["carbohydrate", "corn", "maiz"],
                        "city": ["Guayaquil", "Samborondón"]
                    },
                    {
                        "origin": "USA",
                        "people": {
                            "name": "Katz",
                            "meal": "nurepa"
                        },
                        "code": { "id": 2, "references": 1},
                        "tags": ["carbohydrate", "shell food", "amigos flavor"],
                        "city": ["Oregon", "Brooklin"]
                    }
                ]
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.json | flatten city | where people.name == Katz | count"
        );

        assert_eq!(actual.out, "2");
    })
}

#[test]
fn flatten_more_than_one_column_that_are_subtables_not_supported() {
    Playground::setup("flatten_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "katz.json",
            r#"
                [
                    {
                        "origin": "Ecuador",
                        "people": {
                            "name": "Andres",
                            "meal": "arepa"
                        },
                        "code": { "id": 1, "references": 2},
                        "tags": ["carbohydrate", "corn", "maiz"],
                        "city": ["Guayaquil", "Samborondón"]
                    },
                    {
                        "origin": "USA",
                        "people": {
                            "name": "Katz",
                            "meal": "nurepa"
                        },
                        "code": { "id": 2, "references": 1},
                        "tags": ["carbohydrate", "shell food", "amigos flavor"],
                        "city": ["Oregon", "Brooklin"]
                    }
                ]
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "open katz.json | flatten tags city"
        );

        assert!(actual.err.contains("tried flattening"));
        assert!(actual.err.contains("but is flattened already"));
    })
}
