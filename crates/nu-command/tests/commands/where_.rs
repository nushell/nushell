use nu_test_support::nu;
#[cfg(feature = "database")]
use nu_test_support::pipeline;

#[test]
fn filters_by_unit_size_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | where size > 1kib | sort-by size | get name | first | str trim"
    );

    assert_eq!(actual.out, "cargo_sample.toml");
}

#[test]
fn filters_with_nothing_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"'[{"foo": 3}, {"foo": null}, {"foo": 4}]' | from json | get foo | compact | where $it > 1 | math sum"#
    );

    assert_eq!(actual.out, "7");
}

#[test]
fn filters_with_0_arity_block() {
    let actual = nu!(
        cwd: ".",
        "[1 2 3 4] | where { $in < 3 } | to nuon"
    );

    assert_eq!(actual.out, "[1, 2]");
}

#[test]
fn filters_with_1_arity_block() {
    let actual = nu!(
        cwd: ".",
        "[1 2 3 6 7 8] | where {|e| $e < 5 } | to nuon"
    );

    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn unique_env_each_iteration() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "[1 2] | where { print ($env.PWD | str ends-with 'formats') | cd '/' | true } | to nuon"
    );

    assert_eq!(actual.out, "truetrue[1, 2]");
}

#[test]
fn where_in_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"'[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]' | from json | where name in ["foo"] | get size | math sum"#
    );

    assert_eq!(actual.out, "5");
}

#[test]
fn where_not_in_table() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"'[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]' | from json | where name not-in ["foo"] | get size | math sum"#
    );

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "database")]
#[test]
fn binary_operator_comparisons() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get ints
            | first 4
            | where z > 4200
            | get z.0
        "#
    ));

    assert_eq!(actual.out, "4253");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get ints
            | first 4
            | where z >= 4253
            | get z.0
        "#
    ));

    assert_eq!(actual.out, "4253");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get ints
            | first 4
            | where z < 10
            | get z.0
        "#
    ));

    assert_eq!(actual.out, "1");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get ints
            | first 4
            | where z <= 1
            | get z.0
        "#
    ));

    assert_eq!(actual.out, "1");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get ints
            | where z != 1
            | first
            | get z
        "#
    ));

    assert_eq!(actual.out, "42");
}

#[cfg(feature = "database")]
#[test]
fn contains_operator() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get strings
            | where x =~ ell
            | length
        "#
    ));

    assert_eq!(actual.out, "4");

    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get strings
            | where x !~ ell
            | length
        "#
    ));

    assert_eq!(actual.out, "2");
}
