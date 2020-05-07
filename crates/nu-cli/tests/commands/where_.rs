use nu_test_support::{nu, pipeline};

#[test]
fn filters_by_unit_size_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | where size > 1kb | sort-by size | get name | first 1 | trim | echo $it"
    );

    assert_eq!(actual.out, "cargo_sample.toml");
}

#[test]
fn filters_with_nothing_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"echo '[{"foo": 3}, {"foo": null}, {"foo": 4}]' | from json | where foo > 1 | get foo | sum | echo $it"#
    );

    assert_eq!(actual.out, "7");
}

#[test]
fn where_in_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"echo '[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]' | from json | where name in: ["foo"] | get size | sum | echo $it"#
    );

    assert_eq!(actual.out, "5");
}

#[test]
fn where_not_in_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"echo '[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]' | from json | where name not-in: ["foo"] | get size | sum | echo $it"#
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn explicit_block_condition() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | where table_name == ints
            | get table_values
            | first 4
            | where {= $it.z > 4200}
            | get z
            | echo $it
        "#
    ));

    assert_eq!(actual.out, "4253");
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

    assert_eq!(actual.out, "4253");

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

    assert_eq!(actual.out, "4253");

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

    assert_eq!(actual.out, "1");

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

    assert_eq!(actual.out, "1");

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

    assert_eq!(actual.out, "42");
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

    assert_eq!(actual.out, "4");

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

    assert_eq!(actual.out, "2");
}
