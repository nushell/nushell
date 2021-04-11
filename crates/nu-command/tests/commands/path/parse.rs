use nu_test_support::{nu, pipeline};

use super::join_path_sep;

#[test]
fn parses_single_path_parent() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo 'home/viking/spam.txt'
            | path parse
            | get parent
        "#
    ));

    let expected = join_path_sep(&["home", "viking"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn parses_single_path_stem() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo 'home/viking/spam.txt'
            | path parse
            | get stem
        "#
    ));

    assert_eq!(actual.out, "spam");
}

#[test]
fn parses_column_path_extension() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo [[home, barn]; ['home/viking/spam.txt', 'barn/cow/moo.png']]
            | path parse home barn
            | get barn
            | get extension
        "#
    ));

    assert_eq!(actual.out, "png");
}

#[test]
fn parses_into_correct_number_of_columns() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo 'home/viking/spam.txt'
            | path parse
            | pivot
            | get Column0
            | length
        "#
    ));

    #[cfg(windows)]
    let expected = "4";
    #[cfg(not(windows))]
    let expected = "3";

    assert_eq!(actual.out, expected);
}
