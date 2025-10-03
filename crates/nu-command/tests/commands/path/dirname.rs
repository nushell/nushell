use nu_test_support::nu;

use super::join_path_sep;

#[test]
fn returns_dirname_of_empty_input() {
    let actual = nu!(cwd: "tests", r#"
        echo ""
        | path dirname
    "#);

    assert_eq!(actual.out, "");
}

#[test]
fn replaces_dirname_of_empty_input() {
    let actual = nu!(cwd: "tests", r#"
        echo ""
        | path dirname --replace newdir
    "#);

    assert_eq!(actual.out, "newdir");
}

#[test]
fn returns_dirname_of_path_ending_with_dot() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/."
        | path dirname
    "#);

    assert_eq!(actual.out, "some");
}

#[test]
fn replaces_dirname_of_path_ending_with_dot() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/."
        | path dirname --replace eggs
    "#);

    let expected = join_path_sep(&["eggs", "dir"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_dirname_of_path_ending_with_double_dot() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/.."
        | path dirname
    "#);

    assert_eq!(actual.out, "some/dir");
}

#[test]
fn replaces_dirname_of_path_with_double_dot() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/.."
        | path dirname --replace eggs
    "#);

    let expected = join_path_sep(&["eggs", ".."]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_dirname_of_zero_levels() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/with/spam.txt"
        | path dirname --num-levels 0
    "#);

    assert_eq!(actual.out, "some/dir/with/spam.txt");
}

#[test]
fn replaces_dirname_of_zero_levels_with_empty_string() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/with/spam.txt"
        | path dirname --num-levels 0 --replace ""
    "#);

    assert_eq!(actual.out, "");
}

#[test]
fn replaces_dirname_of_more_levels() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/with/spam.txt"
        | path dirname --replace eggs -n 2
    "#);

    let expected = join_path_sep(&["eggs", "with/spam.txt"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn replaces_dirname_of_way_too_many_levels() {
    let actual = nu!(cwd: "tests", r#"
        echo "some/dir/with/spam.txt"
        | path dirname --replace eggs -n 999
    "#);

    let expected = join_path_sep(&["eggs", "some/dir/with/spam.txt"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn const_path_dirname() {
    let actual = nu!("const name = ('spam/eggs.txt' | path dirname); $name");
    assert_eq!(actual.out, "spam");
}
