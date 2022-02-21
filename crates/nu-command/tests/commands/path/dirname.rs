use nu_test_support::{nu, pipeline};

use super::join_path_sep;

#[test]
fn returns_dirname_of_empty_input() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo ""
            | path dirname
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn replaces_dirname_of_empty_input() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo ""
            | path dirname -r newdir
        "#
    ));

    assert_eq!(actual.out, "newdir");
}

#[test]
fn returns_dirname_of_path_ending_with_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/."
            | path dirname
        "#
    ));

    assert_eq!(actual.out, "some");
}

#[test]
fn replaces_dirname_of_path_ending_with_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/."
            | path dirname -r eggs
        "#
    ));

    let expected = join_path_sep(&["eggs", "dir"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_dirname_of_path_ending_with_double_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/.."
            | path dirname
        "#
    ));

    assert_eq!(actual.out, "some/dir");
}

#[test]
fn replaces_dirname_of_path_with_double_dot() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/.."
            | path dirname -r eggs
        "#
    ));

    let expected = join_path_sep(&["eggs", ".."]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_dirname_of_zero_levels() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/with/spam.txt"
            | path dirname -n 0
        "#
    ));

    assert_eq!(actual.out, "some/dir/with/spam.txt");
}

#[test]
fn replaces_dirname_of_zero_levels_with_empty_string() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/with/spam.txt"
            | path dirname -n 0 -r ""
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn replaces_dirname_of_more_levels() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/with/spam.txt"
            | path dirname -r eggs -n 2
        "#
    ));

    let expected = join_path_sep(&["eggs", "with/spam.txt"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn replaces_dirname_of_way_too_many_levels() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "some/dir/with/spam.txt"
            | path dirname -r eggs -n 999
        "#
    ));

    let expected = join_path_sep(&["eggs", "some/dir/with/spam.txt"]);
    assert_eq!(actual.out, expected);
}
