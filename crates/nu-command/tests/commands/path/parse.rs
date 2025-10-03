use nu_test_support::nu;

#[cfg(windows)]
#[test]
fn parses_single_path_prefix() {
    let actual = nu!(cwd: "tests", r"
        echo 'C:\users\viking\spam.txt'
        | path parse
        | get prefix
    ");

    assert_eq!(actual.out, "C:");
}

#[test]
fn parses_single_path_parent() {
    let actual = nu!(cwd: "tests", r#"
        echo 'home/viking/spam.txt'
        | path parse
        | get parent
    "#);

    assert_eq!(actual.out, "home/viking");
}

#[test]
fn parses_single_path_stem() {
    let actual = nu!(cwd: "tests", r#"
        echo 'home/viking/spam.txt'
        | path parse
        | get stem
    "#);

    assert_eq!(actual.out, "spam");
}

#[test]
fn parses_custom_extension_gets_extension() {
    let actual = nu!(cwd: "tests", r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension tar.gz
        | get extension
    "#);

    assert_eq!(actual.out, "tar.gz");
}

#[test]
fn parses_custom_extension_gets_stem() {
    let actual = nu!(cwd: "tests", r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension tar.gz
        | get stem
    "#);

    assert_eq!(actual.out, "spam");
}

#[test]
fn parses_ignoring_extension_gets_extension() {
    let actual = nu!(cwd: "tests", r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension ''
        | get extension
    "#);

    assert_eq!(actual.out, "");
}

#[test]
fn parses_ignoring_extension_gets_stem() {
    let actual = nu!(cwd: "tests", r#"
        echo 'home/viking/spam.tar.gz'
        | path parse --extension ""
        | get stem
    "#);

    assert_eq!(actual.out, "spam.tar.gz");
}

#[test]
fn parses_into_correct_number_of_columns() {
    let actual = nu!(cwd: "tests", r#"
        echo 'home/viking/spam.txt'
        | path parse
        | transpose
        | get column0
        | length
    "#);

    #[cfg(windows)]
    let expected = "4";
    #[cfg(not(windows))]
    let expected = "3";

    assert_eq!(actual.out, expected);
}

#[test]
fn const_path_parse() {
    let actual = nu!("const name = ('spam/eggs.txt' | path parse); $name.parent");
    assert_eq!(actual.out, "spam");

    let actual = nu!("const name = ('spam/eggs.txt' | path parse); $name.stem");
    assert_eq!(actual.out, "eggs");

    let actual = nu!("const name = ('spam/eggs.txt' | path parse); $name.extension");
    assert_eq!(actual.out, "txt");
}
