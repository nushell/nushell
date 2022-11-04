use std::path::PathBuf;

use nu_test_support::playground::Playground;

use nu_path::expand_path_with;

#[cfg(not(windows))]
#[test]
fn expand_path_with_and_without_relative() {
    let relative_to = "/foo/bar";
    let path = "../..";
    let full_path = "/foo/bar/../..";

    let cwd = std::env::current_dir().expect("Could not get current directory");
    assert_eq!(
        expand_path_with(full_path, cwd),
        expand_path_with(path, relative_to),
    );
}

#[test]
fn expand_path_with_relative() {
    let relative_to = "/foo/bar";
    let path = "../..";

    assert_eq!(PathBuf::from("/"), expand_path_with(path, relative_to),);
}

#[cfg(not(windows))]
#[test]
fn expand_path_no_change() {
    let path = "/foo/bar";

    let cwd = std::env::current_dir().expect("Could not get current directory");
    let actual = expand_path_with(path, cwd);

    assert_eq!(actual, PathBuf::from(path));
}

#[test]
fn expand_unicode_path_no_change() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut spam = dirs.test().clone();
        spam.push("🚒.txt");

        let cwd = std::env::current_dir().expect("Could not get current directory");
        let actual = expand_path_with(spam, cwd);
        let mut expected = dirs.test().clone();
        expected.push("🚒.txt");

        assert_eq!(actual, expected);
    });
}

#[ignore]
#[test]
fn expand_non_utf8_path() {
    // TODO
}

#[test]
fn expand_path_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with("spam.txt", dirs.test());
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_unicode_path_relative_to_unicode_path_with_spaces() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut relative_to = dirs.test().clone();
        relative_to.push("e-$ èрт🚒♞中片-j");

        let actual = expand_path_with("🚒.txt", relative_to);
        let mut expected = dirs.test().clone();
        expected.push("e-$ èрт🚒♞中片-j/🚒.txt");

        assert_eq!(actual, expected);
    });
}

#[ignore]
#[test]
fn expand_non_utf8_path_relative_to_non_utf8_path_with_spaces() {
    // TODO
}

#[test]
fn expand_absolute_path_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut absolute_path = dirs.test().clone();
        absolute_path.push("spam.txt");

        let actual = expand_path_with(&absolute_path, "non/existent/directory");
        let expected = absolute_path;

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_dot_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with("./spam.txt", dirs.test());
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_many_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with("././/.//////./././//.////spam.txt", dirs.test());
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_double_dot_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with("foo/../spam.txt", dirs.test());
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_many_double_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with("foo/bar/baz/../../../spam.txt", dirs.test());
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_3_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with("foo/bar/.../spam.txt", dirs.test());
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_many_3_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with(
            "foo/bar/baz/eggs/sausage/bacon/.../.../.../spam.txt",
            dirs.test(),
        );
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_4_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with("foo/bar/baz/..../spam.txt", dirs.test());
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_many_4_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = expand_path_with(
            "foo/bar/baz/eggs/sausage/bacon/..../..../spam.txt",
            dirs.test(),
        );
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_with_way_too_many_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut relative_to = dirs.test().clone();
        relative_to.push("foo/bar/baz/eggs/sausage/bacon/vikings");

        let actual = expand_path_with("././..////././...///././.....///spam.txt", relative_to);
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_unicode_path_with_way_too_many_dots_relative_to_unicode_path_with_spaces() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut relative_to = dirs.test().clone();
        relative_to.push("foo/áčěéí  +šř=é/baz/eggs/e-$ èрт🚒♞中片-j/bacon/öäöä öäöä");

        let actual = expand_path_with("././..////././...///././.....///🚒.txt", relative_to);
        let mut expected = dirs.test().clone();
        expected.push("🚒.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn expand_path_tilde() {
    let tilde_path = "~";

    let cwd = std::env::current_dir().expect("Could not get current directory");
    let actual = expand_path_with(tilde_path, cwd);

    assert!(actual.is_absolute());
    assert!(!actual.starts_with("~"));
}

#[test]
fn expand_path_tilde_relative_to() {
    let tilde_path = "~";

    let actual = expand_path_with(tilde_path, "non/existent/path");

    assert!(actual.is_absolute());
    assert!(!actual.starts_with("~"));
}
