use nu_test_support::prelude::*;

#[cfg(windows)]
#[test]
fn parses_single_path_prefix() -> Result {
    let code = r#"
        echo 'C:\users\viking\spam.txt'
        | path parse
        | get prefix
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "C:");
    Ok(())
}

#[test]
fn parses_single_path_parent() -> Result {
    let code = r#"
        echo 'home/viking/spam.txt'
        | path parse
        | get parent
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "home/viking");
    Ok(())
}

#[test]
fn parses_single_path_stem() -> Result {
    let code = r#"
        echo 'home/viking/spam.txt'
        | path parse
        | get stem
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "spam");
    Ok(())
}

#[test]
fn parses_custom_extension_gets_extension() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension tar.gz
        | get extension
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "tar.gz");
    Ok(())
}

#[test]
fn parses_custom_extension_gets_stem() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension tar.gz
        | get stem
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "spam");
    Ok(())
}

#[test]
fn parses_ignoring_extension_gets_extension() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension ''
        | get extension
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "");
    Ok(())
}

#[test]
fn parses_ignoring_extension_gets_stem() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension ""
        | get stem
    "#;

    let outcome: String = test().cwd("tests").run(code)?;
    assert_eq!(outcome, "spam.tar.gz");
    Ok(())
}

#[test]
fn parses_into_correct_number_of_columns() -> Result {
    let code = r#"
        echo 'home/viking/spam.txt'
        | path parse
        | transpose
        | get column0
        | length
    "#;

    #[cfg(windows)]
    let expected = 4;
    #[cfg(not(windows))]
    let expected = 3;

    let outcome: i64 = test().cwd("tests").run(code)?;
    assert_eq!(outcome, expected);
    Ok(())
}

#[test]
fn const_path_parse() -> Result {
    let code = "const name = ('spam/eggs.txt' | path parse); $name.parent";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "spam");

    let code = "const name = ('spam/eggs.txt' | path parse); $name.stem";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "eggs");

    let code = "const name = ('spam/eggs.txt' | path parse); $name.extension";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "txt");
    Ok(())
}
