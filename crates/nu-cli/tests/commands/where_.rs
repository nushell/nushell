use nu_test_support::{nu, pipeline};

#[test]
fn filters_by_unit_size_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | where size > 1kb | sort-by size | get name | first 1 | trim | echo $it"
    );

    assert_eq!(actual, "cargo_sample.toml");
}

#[test]
fn binary_operator_comparisons() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == ints
            | get table_values
            | first 4
            | where z > 4200
            | get z
            | echo $it
        "#
    ));

    assert_eq!(actual, "4253");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == ints
            | get table_values
            | first 4
            | where z >= 4253
            | get z
            | echo $it
        "#
    ));

    assert_eq!(actual, "4253");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == ints
            | get table_values
            | first 4
            | where z < 10
            | get z
            | echo $it
        "#
    ));

    assert_eq!(actual, "1");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == ints
            | get table_values
            | first 4
            | where z <= 1
            | get z
            | echo $it
        "#
    ));

    assert_eq!(actual, "1");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == ints
            | get table_values
            | where z != 1
            | first 1
            | get z
            | echo $it
        "#
    ));

    assert_eq!(actual, "42");
}

#[test]
fn contains_operator() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == strings
            | get table_values
            | where x =~ ell
            | count
            | echo $it
        "#
    ));

    assert_eq!(actual, "4");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == strings
            | get table_values
            | where x !~ ell
            | count
            | echo $it
        "#
    ));

    assert_eq!(actual, "2");
}
