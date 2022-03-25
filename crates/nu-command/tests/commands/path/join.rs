use nu_test_support::{nu, pipeline};

use super::join_path_sep;

#[test]
fn returns_path_joined_with_column_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo [ [name]; [eggs] ]
            | path join spam.txt -c [ name ]
            | get name.0
        "#
    ));

    let expected = join_path_sep(&["eggs", "spam.txt"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_path_joined_from_list() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo [ home viking spam.txt ]
            | path join
        "#
    ));

    let expected = join_path_sep(&["home", "viking", "spam.txt"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn drop_one_path_join() {
    let actual = nu!(
        cwd: "tests", pipeline(
            r#"[a, b, c] | drop 1 | path join
        "#
    ));

    let expected = join_path_sep(&["a", "b"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn appends_slash_when_joined_with_empty_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "/some/dir"
            | path join ''
        "#
    ));

    let expected = join_path_sep(&["/some/dir", ""]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_joined_path_when_joining_empty_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo ""
            | path join foo.txt
        "#
    ));

    assert_eq!(actual.out, "foo.txt");
}
