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
            | sort-by column1
            | skip 1
            | first 1
            | get column1
            | str trim
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
            | get column1
            | str trim
        "#
    ));

    assert!(actual.err.contains("Cannot find column"));
    assert!(actual.err.contains("value originates here"));
}

#[test]
fn by_invalid_types() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | echo ["foo" 1]
            | sort-by
            | to json -r
        "#
    ));

    let json_output = r#"[1,"foo"]"#;
    assert_eq!(actual.out, json_output);
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
        "#
    ));

    assert_eq!(actual.out, "authors = [\"The Nushell Project Developers\"]");
}

#[test]
fn ls_sort_by_name_sensitive() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample-ls-output.json
            | sort-by name
            | select name
            | to json --raw
        "#
    ));

    let json_output = r#"[{"name": "B.txt"},{"name": "C"},{"name": "a.txt"}]"#;

    assert_eq!(actual.out, json_output);
}

#[test]
fn ls_sort_by_name_insensitive() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample-ls-output.json
            | sort-by -i name
            | select name
            | to json --raw
        "#
    ));

    let json_output = r#"[{"name": "a.txt"},{"name": "B.txt"},{"name": "C"}]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn ls_sort_by_type_name_sensitive() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample-ls-output.json
            | sort-by type name
            | select name type
            | to json --raw
        "#
    ));

    let json_output = r#"[{"name": "C","type": "Dir"},{"name": "B.txt","type": "File"},{"name": "a.txt","type": "File"}]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn ls_sort_by_type_name_insensitive() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample-ls-output.json
            | sort-by -i type name
            | select name type
            | to json --raw
        "#
    ));

    let json_output = r#"[{"name": "C","type": "Dir"},{"name": "a.txt","type": "File"},{"name": "B.txt","type": "File"}]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn sort_different_types() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            [a, 1, b, 2, c, 3, [4, 5, 6], d, 4, [1, 2, 3]] | sort-by | to json --raw
        "#
    ));

    let json_output = r#"[1,2,3,4,"a","b","c","d",[1,2,3],[4,5,6]]"#;
    assert_eq!(actual.out, json_output);
}
