use nu_test_support::{nu, pipeline};

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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
            | str trim
        "#
    ));

    assert_eq!(actual.out, "description");
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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
            | str trim
        "#
    ));

    assert!(actual.err.contains("Can not find column to sort by"));
    assert!(actual.err.contains("invalid column"));
}

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn by_invalid_types() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | echo [1 "foo"]
            | sort-by
        "#
    ));

    assert!(actual.err.contains("Not all values can be compared"));
    assert!(actual
        .err
        .contains("Unable to sort values, as \"integer\" cannot compare against \"string\""));
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
<<<<<<< HEAD
            | to json
        "#
    ));

    let json_output = r#"[{"name":"B.txt"},{"name":"C"},{"name":"a.txt"}]"#;
=======
            | to json --raw
        "#
    ));

    //let json_output = r#"[{"name":"B.txt"},{"name":"C"},{"name":"a.txt"}]"#;
    let json_output = r#"[{"name": "B.txt"},{"name": "C"},{"name": "a.txt"}]"#;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce

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
<<<<<<< HEAD
            | to json
        "#
    ));

    let json_output = r#"[{"name":"a.txt"},{"name":"B.txt"},{"name":"C"}]"#;

=======
            | to json --raw
        "#
    ));

    let json_output = r#"[{"name": "B.txt"},{"name": "C"},{"name": "a.txt"}]"#;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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
<<<<<<< HEAD
            | to json
        "#
    ));

    let json_output = r#"[{"name":"C","type":"Dir"},{"name":"B.txt","type":"File"},{"name":"a.txt","type":"File"}]"#;

=======
            | to json --raw
        "#
    ));

    let json_output = r#"[{"name": "C","type": "Dir"},{"name": "a.txt","type": "File"},{"name": "B.txt","type": "File"}]"#;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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
<<<<<<< HEAD
            | to json
        "#
    ));

    let json_output = r#"[{"name":"C","type":"Dir"},{"name":"a.txt","type":"File"},{"name":"B.txt","type":"File"}]"#;

=======
            | to json --raw
        "#
    ));

    let json_output = r#"[{"name": "C","type": "Dir"},{"name": "a.txt","type": "File"},{"name": "B.txt","type": "File"}]"#;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    assert_eq!(actual.out, json_output);
}
