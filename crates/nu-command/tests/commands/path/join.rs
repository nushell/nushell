use nu_test_support::{nu, pipeline};

use super::join_path_sep;

#[test]
fn returns_path_joined_with_column_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo [ [name]; [eggs] ]
            | path join -a spam.txt name
            | get name
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
fn appends_slash_when_joined_with_empty_path() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "/some/dir"
            | path join -a ''
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
            | path join -a foo.txt
        "#
    ));

    assert_eq!(actual.out, "foo.txt");
}
