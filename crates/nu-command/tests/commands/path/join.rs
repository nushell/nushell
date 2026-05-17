use nu_test_support::prelude::*;

use super::join_path_sep;

#[test]
fn returns_path_joined_from_list() -> Result {
    let code = "
        echo [ home viking spam.txt ]
        | path join
    ";

    let expected = join_path_sep(&["home", "viking", "spam.txt"]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn drop_one_path_join() -> Result {
    let code = "[a, b, c] | drop 1 | path join
            ";

    let expected = join_path_sep(&["a", "b"]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn appends_slash_when_joined_with_empty_path() -> Result {
    let code = r#"
        echo "/some/dir"
        | path join ''
    "#;

    let expected = join_path_sep(&["/some/dir", ""]);
    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn returns_joined_path_when_joining_empty_path() -> Result {
    let code = r#"
        echo ""
        | path join foo.txt
    "#;

    test().cwd("tests").run(code).expect_value_eq("foo.txt")
}

#[test]
fn const_path_join() -> Result {
    let code = "const name = ('spam' | path join 'eggs.txt'); $name";
    let expected = join_path_sep(&["spam", "eggs.txt"]);
    test().run(code).expect_value_eq(expected)
}
