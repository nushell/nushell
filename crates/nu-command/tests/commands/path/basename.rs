use nu_test_support::prelude::*;

use super::join_path_sep;

#[test]
fn returns_basename_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path basename
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "");
    Ok(())
}

#[test]
fn replaces_basename_of_empty_input() -> Result {
    let code = r#"
        echo ""
        | path basename --replace newname.txt
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "newname.txt");
    Ok(())
}

#[test]
fn returns_basename_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/file.txt/."
        | path basename
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "file.txt");
    Ok(())
}

#[test]
fn replaces_basename_of_path_ending_with_dot() -> Result {
    let code = r#"
        echo "some/file.txt/."
        | path basename --replace viking.txt
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["some", "viking.txt"]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn returns_basename_of_path_ending_with_double_dot() -> Result {
    let code = r#"
        echo "some/file.txt/.."
        | path basename
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "");
    Ok(())
}

#[test]
fn replaces_basename_of_path_ending_with_double_dot() -> Result {
    let code = r#"
        echo "some/file.txt/.."
        | path basename --replace eggs
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["some/file.txt/..", "eggs"]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn const_path_basename() -> Result {
    let code = "const name = ('spam/eggs.txt' | path basename); $name";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "eggs.txt");
    Ok(())
}
