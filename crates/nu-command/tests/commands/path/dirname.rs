use nu_test_support::prelude::*;

use super::join_path_sep;

#[test]
fn returns_dirname_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path dirname
    "#;

    test().cwd("tests").run(code).expect_value_eq("")
}

#[test]
fn replaces_dirname_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path dirname --replace newdir
    "#;

    test().cwd("tests").run(code).expect_value_eq("newdir")
}

#[test]
fn returns_dirname_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/dir/."
        | path dirname
    "#;

    test().cwd("tests").run(code).expect_value_eq("some")
}

#[test]
fn replaces_dirname_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/dir/."
        | path dirname --replace eggs
    "#;

    let expected = join_path_sep(&["eggs", "dir"]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn returns_dirname_of_path_ending_with_double_dot() -> Result {
    let code = r#"
        echo "some/dir/.."
        | path dirname
    "#;

    test().cwd("tests").run(code).expect_value_eq("some/dir")
}

#[test]
fn replaces_dirname_of_path_with_double_dot() -> Result {
    let code = r#"
        echo "some/dir/.."
        | path dirname --replace eggs
    "#;

    let expected = join_path_sep(&["eggs", ".."]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn returns_dirname_of_zero_levels() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --num-levels 0
    "#;

    test()
        .cwd("tests")
        .run(code)
        .expect_value_eq("some/dir/with/spam.txt")
}

#[test]
fn replaces_dirname_of_zero_levels_with_empty_string() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --num-levels 0 --replace ""
    "#;

    test().cwd("tests").run(code).expect_value_eq("")
}

#[test]
fn replaces_dirname_of_more_levels() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --replace eggs -n 2
    "#;

    let expected = join_path_sep(&["eggs", "with/spam.txt"]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn replaces_dirname_of_way_too_many_levels() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --replace eggs -n 999
    "#;

    let expected = join_path_sep(&["eggs", "some/dir/with/spam.txt"]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn const_path_dirname() -> Result {
    let code = "const name = ('spam/eggs.txt' | path dirname); $name";
    test().run(code).expect_value_eq("spam")
}
