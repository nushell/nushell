use nu_test_support::prelude::*;

use super::join_path_sep;

#[test]
fn returns_path_joined_from_list() -> Result {
    let code = r#"
        echo [ home viking spam.txt ]
        | path join
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["home", "viking", "spam.txt"]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn drop_one_path_join() -> Result {
    let code = r#"[a, b, c] | drop 1 | path join
            "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["a", "b"]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn appends_slash_when_joined_with_empty_path() -> Result {
    let code = r#"
        echo "/some/dir"
        | path join ''
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    let expected = join_path_sep(&["/some/dir", ""]);
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn returns_joined_path_when_joining_empty_path() -> Result {
    let code = r#"
        echo ""
        | path join foo.txt
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "foo.txt");
    Ok(())
}

#[test]
fn const_path_join() -> Result {
    let code = "const name = ('spam' | path join 'eggs.txt'); $name";
    let outcome: String = test().run(code)?;
    let expected = join_path_sep(&["spam", "eggs.txt"]);
    assert_eq!(outcome, expected);
    Ok(())
}
