use nu_test_support::prelude::*;

#[cfg(windows)]
#[test]
fn parses_single_path_prefix() -> Result {
    let code = r#"
        echo 'C:\users\viking\spam.txt'
        | path parse
        | get prefix
    "#;

    test().cwd("tests").run(code).expect_value_eq("C:")
}

#[test]
fn parses_single_path_parent() -> Result {
    let code = r#"
        echo 'home/viking/spam.txt'
        | path parse
        | get parent
    "#;

    test().cwd("tests").run(code).expect_value_eq("home/viking")
}

#[test]
fn parses_single_path_stem() -> Result {
    let code = r#"
        echo 'home/viking/spam.txt'
        | path parse
        | get stem
    "#;

    test().cwd("tests").run(code).expect_value_eq("spam")
}

#[test]
fn parses_custom_extension_gets_extension() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension tar.gz
        | get extension
    "#;

    test().cwd("tests").run(code).expect_value_eq("tar.gz")
}

#[test]
fn parses_custom_extension_gets_stem() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension tar.gz
        | get stem
    "#;

    test().cwd("tests").run(code).expect_value_eq("spam")
}

#[test]
fn parses_ignoring_extension_gets_extension() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension ''
        | get extension
    "#;

    test().cwd("tests").run(code).expect_value_eq("")
}

#[test]
fn parses_ignoring_extension_gets_stem() -> Result {
    let code = r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension ""
        | get stem
    "#;

    test().cwd("tests").run(code).expect_value_eq("spam.tar.gz")
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

    test().cwd("tests").run(code).expect_value_eq(expected)
}

#[test]
fn const_path_parse() -> Result {
    let code = "const name = ('spam/eggs.txt' | path parse); $name.parent";
    test().run(code).expect_value_eq("spam")?;

    let code = "const name = ('spam/eggs.txt' | path parse); $name.stem";
    test().run(code).expect_value_eq("eggs")?;

    let code = "const name = ('spam/eggs.txt' | path parse); $name.extension";
    test().run(code).expect_value_eq("txt")?;
    Ok(())
}
