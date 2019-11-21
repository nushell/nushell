mod helpers;

use helpers as h;

#[test]
fn test_compare() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
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
        cwd: "tests/fixtures/formats", h::pipeline(
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
        cwd: "tests/fixtures/formats", h::pipeline(
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
        cwd: "tests/fixtures/formats", h::pipeline(
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
        cwd: "tests/fixtures/formats", h::pipeline(
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
fn test_contains() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", h::pipeline(
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
        cwd: "tests/fixtures/formats", h::pipeline(
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
