use nu_test_support::prelude::*;

use super::join_path_sep;

#[test]
fn returns_basename_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path basename
    "#;

    test().cwd("tests").run(code).expect_value_eq("")
}

#[test]
fn replaces_basename_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path basename --replace newname.txt
    "#;

    test().cwd("tests").run(code).expect_value_eq("newname.txt")
}

#[test]
fn returns_basename_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/file.txt/."
        | path basename
    "#;

    test().cwd("tests").run(code).expect_value_eq("file.txt")
}

#[test]
fn replaces_basename_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/file.txt/."
        | path basename --replace viking.txt
    "#;

    let expected = join_path_sep(&["some", "viking.txt"]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn returns_basename_of_path_ending_with_double_dot() -> Result {
    let code = r#"
        echo "some/file.txt/.."
        | path basename
    "#;

    test().cwd("tests").run(code).expect_value_eq("")
}

#[test]
fn replaces_basename_of_path_ending_with_double_dot() -> Result {
    let code = r#"
        echo "some/file.txt/.."
        | path basename --replace eggs
    "#;

    let expected = join_path_sep(&["some/file.txt/..", "eggs"]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn const_path_basename() -> Result {
    let code = "const name = ('spam/eggs.txt' | path basename); $name";
    test().run(code).expect_value_eq("eggs.txt")
}
