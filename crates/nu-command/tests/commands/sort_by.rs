use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn by_column() -> Result {
    let code = r#"
        open cargo_sample.toml --raw
        | lines
        | skip 1
        | first 4
        | split column "="
        | sort-by column0
        | skip 1
        | first
        | get column0
        | str trim
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("description")
}

#[test]
fn by_invalid_column() -> Result {
    let code = r#"
        open cargo_sample.toml --raw
        | lines
        | skip 1
        | first 4
        | split column "="
        | sort-by ColumnThatDoesNotExist
        | skip 1
        | first
        | get column0
        | str trim
    "#;

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantFindColumn { col_name, .. } if col_name == "ColumnThatDoesNotExist"
    );
    Ok(())
}

#[test]
fn sort_by_empty() -> Result {
    test()
        .run("[] | sort-by foo")
        .expect_value_eq(Vec::<String>::new())
}

#[test]
fn ls_sort_by_name_sensitive() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("open sample-ls-output.json | sort-by name | select name")
        .expect_value_eq(test_table![
            ["name"];
            ["B.txt"],
            ["C"],
            ["a.txt"],
        ])
}

#[test]
fn ls_sort_by_name_insensitive() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("open sample-ls-output.json | sort-by -i name | select name")
        .expect_value_eq(test_table![
            ["name"];
            ["a.txt"],
            ["B.txt"],
            ["C"],
        ])
}

#[test]
fn ls_sort_by_type_name_sensitive() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("open sample-ls-output.json | sort-by type name | select name type")
        .expect_value_eq(test_table![
            ["name", "type"];
            ["C", "Dir"],
            ["B.txt", "File"],
            ["a.txt", "File"],
        ])
}

#[test]
fn ls_sort_by_type_name_insensitive() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("open sample-ls-output.json | sort-by -i type name | select name type")
        .expect_value_eq(test_table![
            ["name", "type"];
            ["C", "Dir"],
            ["a.txt", "File"],
            ["B.txt", "File"],
        ])
}

#[test]
fn no_column_specified_fails() -> Result {
    test()
        .run("[2 0 1] | sort-by")
        .expect_error_code_eq("nu::shell::missing_parameter")
}

#[test]
fn fail_on_non_iterator() -> Result {
    test()
        .run("1 | sort-by")
        .expect_error_code_eq("nu::parser::input_type_mismatch")
}
