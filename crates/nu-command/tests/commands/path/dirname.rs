use nu_test_support::prelude::*;

use super::join_path_sep;

#[test]
fn returns_dirname_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path dirname
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "");
    Ok(())
}

#[test]
fn replaces_dirname_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path dirname --replace newdir
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "newdir");
    Ok(())
}

#[test]
fn returns_dirname_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/dir/."
        | path dirname
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "some");
    Ok(())
}

#[test]
fn replaces_dirname_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/dir/."
        | path dirname --replace eggs
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["eggs", "dir"]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn returns_dirname_of_path_ending_with_double_dot() -> Result {
    let code = r#"
        echo "some/dir/.."
        | path dirname
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "some/dir");
    Ok(())
}

#[test]
fn replaces_dirname_of_path_with_double_dot() -> Result {
    let code = r#"
        echo "some/dir/.."
        | path dirname --replace eggs
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["eggs", ".."]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn returns_dirname_of_zero_levels() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --num-levels 0
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "some/dir/with/spam.txt");
    Ok(())
}

#[test]
fn replaces_dirname_of_zero_levels_with_empty_string() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --num-levels 0 --replace ""
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "");
    Ok(())
}

#[test]
fn replaces_dirname_of_more_levels() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --replace eggs -n 2
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["eggs", "with/spam.txt"]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn replaces_dirname_of_way_too_many_levels() -> Result {
    let code = r#"
        echo "some/dir/with/spam.txt"
        | path dirname --replace eggs -n 999
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["eggs", "some/dir/with/spam.txt"]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn const_path_dirname() -> Result {
    let code = "const name = ('spam/eggs.txt' | path dirname); $name";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "spam");
    Ok(())
}
