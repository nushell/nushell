use nu_test_support::{nu, pipeline};

use super::join_path_sep;

#[test]
fn returns_basename_of_empty_input() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo ""
            | path basename
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn replaces_basename_of_empty_input() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo ""
            | path basename -r newname.txt
        "#
    ));

    assert_eq!(actual.out, "newname.txt");
}

#[test]
fn returns_basename_of_path_ending_with_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/file.txt/."
            | path basename
        "#
    ));

    assert_eq!(actual.out, "file.txt");
}

#[test]
fn replaces_basename_of_path_ending_with_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/file.txt/."
            | path basename -r viking.txt
        "#
    ));

    let expected = join_path_sep(&["some", "viking.txt"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_basename_of_path_ending_with_double_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/file.txt/.."
            | path basename
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn replaces_basename_of_path_ending_with_double_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/file.txt/.."
            | path basename -r eggs
        "#
    ));

    let expected = join_path_sep(&["some/file.txt/..", "eggs"]);
    assert_eq!(actual.out, expected);
}
