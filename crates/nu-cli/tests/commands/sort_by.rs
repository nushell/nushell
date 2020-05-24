use nu_test_support::{nu, pipeline};

#[test]
fn by_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 4
            | split column "="
            | sort-by Column1
            | skip 1
            | first 1
            | get Column1
            | trim
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "description");
}

#[test]
fn by_invalid_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 4
            | split column "="
            | sort-by ColumnThatDoesNotExist
            | skip 1
            | first 1
            | get Column1
            | trim
            | echo $it
        "#
    ));

    assert!(actual.err.contains("Can not find column to sort by"));
    assert!(actual.err.contains("invalid column"));
}

#[test]
fn sort_primitive_values() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 6
            | sort-by
            | first 1
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "authors = [\"The Nu Project Contributors\"]");
}
