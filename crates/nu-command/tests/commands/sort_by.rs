use nu_test_support::nu;

#[test]
fn by_column() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml --raw
        | lines
        | skip 1
        | first 4
        | split column "="
        | sort-by column1
        | skip 1
        | first
        | get column1
        | str trim
    "#);

    assert_eq!(actual.out, "description");
}

#[test]
fn by_invalid_column() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml --raw
        | lines
        | skip 1
        | first 4
        | split column "="
        | sort-by ColumnThatDoesNotExist
        | skip 1
        | first
        | get column1
        | str trim
    "#);

    assert!(actual.err.contains("Cannot find column"));
    assert!(actual.err.contains("value originates here"));
}

#[test]
fn sort_by_empty() {
    let actual = nu!("[] | sort-by foo | to nuon");

    assert_eq!(actual.out, "[]");
}

#[test]
fn ls_sort_by_name_sensitive() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample-ls-output.json
        | sort-by name
        | select name
        | to json --raw
    ");

    let json_output = r#"[{"name":"B.txt"},{"name":"C"},{"name":"a.txt"}]"#;

    assert_eq!(actual.out, json_output);
}

#[test]
fn ls_sort_by_name_insensitive() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample-ls-output.json
        | sort-by -i name
        | select name
        | to json --raw
    ");

    let json_output = r#"[{"name":"a.txt"},{"name":"B.txt"},{"name":"C"}]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn ls_sort_by_type_name_sensitive() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample-ls-output.json
        | sort-by type name
        | select name type
        | to json --raw
    ");

    let json_output = r#"[{"name":"C","type":"Dir"},{"name":"B.txt","type":"File"},{"name":"a.txt","type":"File"}]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn ls_sort_by_type_name_insensitive() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample-ls-output.json
        | sort-by -i type name
        | select name type
        | to json --raw
    ");

    let json_output = r#"[{"name":"C","type":"Dir"},{"name":"a.txt","type":"File"},{"name":"B.txt","type":"File"}]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn no_column_specified_fails() {
    let actual = nu!("[2 0 1] | sort-by");

    assert!(actual.err.contains("missing parameter"));
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | sort-by");

    assert!(actual.err.contains("command doesn't support"));
}
