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

#[test]
fn ls_sort_by_name_sensitive() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample-ls-output.json
            | sort-by name
            | select name
            | to json
        "#
    ));

    let json_output = r#"[{"name":"B.txt"},{"name":"C"},{"name":"a.txt"}]"#;

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
            | to json
        "#
    ));

    let json_output = r#"[{"name":"a.txt"},{"name":"B.txt"},{"name":"C"}]"#;

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
            | to json
        "#
    ));

    let json_output = r#"[{"name":"C","type":"Dir"},{"name":"B.txt","type":"File"},{"name":"a.txt","type":"File"}]"#;

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
            | to json
        "#
    ));

    let json_output = r#"[{"name":"C","type":"Dir"},{"name":"a.txt","type":"File"},{"name":"B.txt","type":"File"}]"#;

    assert_eq!(actual.out, json_output);
}
