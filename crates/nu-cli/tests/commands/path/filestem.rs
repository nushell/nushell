use nu_test_support::{nu, pipeline};

use super::join_path_sep;

#[test]
fn returns_filestem_of_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "menu/eggs/." 
            | path filestem
        "#
    ));

    assert_eq!(actual.out, "eggs");
}

#[test]
fn returns_filestem_of_double_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "menu/eggs/.." 
            | path filestem
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn returns_filestem_of_path_with_empty_prefix() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "menu/spam.txt" 
            | path filestem -p ""
        "#
    ));

    assert_eq!(actual.out, "spam");
}

#[test]
fn returns_filestem_of_path_with_empty_suffix() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "menu/spam.txt" 
            | path filestem -s ""
        "#
    ));

    assert_eq!(actual.out, "spam.txt");
}

#[test]
fn returns_filestem_of_path_with_empty_prefix_and_suffix() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "menu/spam.txt" 
            | path filestem -p "" -s ""
        "#
    ));

    assert_eq!(actual.out, "spam.txt");
}

#[test]
fn returns_filestem_with_wrong_prefix_and_suffix() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "menu/spam.txt" 
            | path filestem -p "bacon" -s "eggs"
        "#
    ));

    assert_eq!(actual.out, "spam.txt");
}

#[test]
fn replaces_filestem_stripped_to_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "menu/spam.txt" 
            | path filestem -p "spam" -s "txt" -r ".eggs."
        "#
    ));

    let expected = join_path_sep(&["menu", "spam.eggs.txt"]);
    assert_eq!(actual.out, expected);
}
