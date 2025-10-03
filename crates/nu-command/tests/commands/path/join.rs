use nu_test_support::nu;

use super::join_path_sep;

#[test]
fn returns_path_joined_from_list() {
    let actual = nu!(cwd: "tests", r#"
        echo [ home viking spam.txt ]
        | path join
    "#);

    let expected = join_path_sep(&["home", "viking", "spam.txt"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn drop_one_path_join() {
    let actual = nu!(cwd: "tests", r#"[a, b, c] | drop 1 | path join
            "#);

    let expected = join_path_sep(&["a", "b"]);
    assert_eq!(actual.out, expected);
}

#[test]
fn appends_slash_when_joined_with_empty_path() {
    let actual = nu!(cwd: "tests", r#"
        echo "/some/dir"
        | path join ''
    "#);

    let expected = join_path_sep(&["/some/dir", ""]);
    assert_eq!(actual.out, expected);
}

#[test]
fn returns_joined_path_when_joining_empty_path() {
    let actual = nu!(cwd: "tests", r#"
        echo ""
        | path join foo.txt
    "#);

    assert_eq!(actual.out, "foo.txt");
}

#[test]
fn const_path_join() {
    let actual = nu!("const name = ('spam' | path join 'eggs.txt'); $name");
    let expected = join_path_sep(&["spam", "eggs.txt"]);
    assert_eq!(actual.out, expected);
}
