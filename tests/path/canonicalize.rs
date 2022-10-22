use std::path::Path;

use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;

use nu_path::canonicalize_with;

#[test]
fn canonicalize_path() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let mut spam = dirs.test().clone();
        spam.push("spam.txt");

        let cwd = std::env::current_dir().expect("Could not get current directory");
        let actual = canonicalize_with(spam, cwd).expect("Failed to canonicalize");

        assert!(actual.ends_with("spam.txt"));
    });
}

#[test]
fn canonicalize_unicode_path() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("🚒.txt")]);

        let mut spam = dirs.test().clone();
        spam.push("🚒.txt");

        let cwd = std::env::current_dir().expect("Could not get current directory");

        let actual = canonicalize_with(spam, cwd).expect("Failed to canonicalize");

        assert!(actual.ends_with("🚒.txt"));
    });
}

#[ignore]
#[test]
fn canonicalize_non_utf8_path() {
    // TODO
}

#[test]
fn canonicalize_path_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = canonicalize_with("spam.txt", dirs.test()).expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_unicode_path_relative_to_unicode_path_with_spaces() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("e-$ èрт🚒♞中片-j");
        sandbox.with_files(vec![EmptyFile("e-$ èрт🚒♞中片-j/🚒.txt")]);

        let mut relative_to = dirs.test().clone();
        relative_to.push("e-$ èрт🚒♞中片-j");

        let actual = canonicalize_with("🚒.txt", relative_to).expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("e-$ èрт🚒♞中片-j/🚒.txt");

        assert_eq!(actual, expected);
    });
}

#[ignore]
#[test]
fn canonicalize_non_utf8_path_relative_to_non_utf8_path_with_spaces() {
    // TODO
}

#[test]
fn canonicalize_absolute_path_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let mut absolute_path = dirs.test().clone();
        absolute_path.push("spam.txt");

        let actual = canonicalize_with(&absolute_path, "non/existent/directory")
            .expect("Failed to canonicalize");
        let expected = absolute_path;

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_dot() {
    let expected = std::env::current_dir().expect("Could not get current directory");

    let actual = canonicalize_with(".", expected.as_path()).expect("Failed to canonicalize");

    assert_eq!(actual, expected);
}

#[test]
fn canonicalize_many_dots() {
    let expected = std::env::current_dir().expect("Could not get current directory");

    let actual = canonicalize_with("././/.//////./././//.///", expected.as_path())
        .expect("Failed to canonicalize");

    assert_eq!(actual, expected);
}

#[test]
fn canonicalize_path_with_dot_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = canonicalize_with("./spam.txt", dirs.test()).expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_path_with_many_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = canonicalize_with("././/.//////./././//.////spam.txt", dirs.test())
            .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_double_dot() {
    let cwd = std::env::current_dir().expect("Could not get current directory");
    let actual = canonicalize_with("..", &cwd).expect("Failed to canonicalize");
    let expected = cwd
        .parent()
        .expect("Could not get parent of current directory");

    assert_eq!(actual, expected);
}

#[test]
fn canonicalize_path_with_double_dot_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo");
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual =
            canonicalize_with("foo/../spam.txt", dirs.test()).expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_path_with_many_double_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar/baz");
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = canonicalize_with("foo/bar/baz/../../../spam.txt", dirs.test())
            .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_ndots() {
    let cwd = std::env::current_dir().expect("Could not get current directory");
    let actual = canonicalize_with("...", &cwd).expect("Failed to canonicalize");
    let expected = cwd
        .parent()
        .expect("Could not get parent of current directory")
        .parent()
        .expect("Could not get parent of a parent of current directory");

    assert_eq!(actual, expected);
}

#[test]
fn canonicalize_path_with_3_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar");
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual =
            canonicalize_with("foo/bar/.../spam.txt", dirs.test()).expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_path_with_many_3_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar/baz/eggs/sausage/bacon");
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = canonicalize_with(
            "foo/bar/baz/eggs/sausage/bacon/.../.../.../spam.txt",
            dirs.test(),
        )
        .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_path_with_4_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar/baz");
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = canonicalize_with("foo/bar/baz/..../spam.txt", dirs.test())
            .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_path_with_many_4_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar/baz/eggs/sausage/bacon");
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = canonicalize_with(
            "foo/bar/baz/eggs/sausage/bacon/..../..../spam.txt",
            dirs.test(),
        )
        .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_path_with_way_too_many_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar/baz/eggs/sausage/bacon/vikings");
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let mut relative_to = dirs.test().clone();
        relative_to.push("foo/bar/baz/eggs/sausage/bacon/vikings");

        let actual = canonicalize_with("././..////././...///././.....///spam.txt", relative_to)
            .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_unicode_path_with_way_too_many_dots_relative_to_unicode_path_with_spaces() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/áčěéí  +šř=é/baz/eggs/e-$ èрт🚒♞中片-j/bacon/öäöä öäöä");
        sandbox.with_files(vec![EmptyFile("🚒.txt")]);

        let mut relative_to = dirs.test().clone();
        relative_to.push("foo/áčěéí  +šř=é/baz/eggs/e-$ èрт🚒♞中片-j/bacon/öäöä öäöä");

        let actual = canonicalize_with("././..////././...///././.....///🚒.txt", relative_to)
            .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("🚒.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_tilde() {
    let tilde_path = "~";

    let cwd = std::env::current_dir().expect("Could not get current directory");
    let actual = canonicalize_with(tilde_path, cwd).expect("Failed to canonicalize");

    assert!(actual.is_absolute());
    assert!(!actual.starts_with("~"));
}

#[test]
fn canonicalize_tilde_relative_to() {
    let tilde_path = "~";

    let actual =
        canonicalize_with(tilde_path, "non/existent/path").expect("Failed to canonicalize");

    assert!(actual.is_absolute());
    assert!(!actual.starts_with("~"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn canonicalize_symlink() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);
        sandbox.symlink("spam.txt", "link_to_spam.txt");

        let mut symlink_path = dirs.test().clone();
        symlink_path.push("link_to_spam.txt");

        let cwd = std::env::current_dir().expect("Could not get current directory");
        let actual = canonicalize_with(symlink_path, cwd).expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn canonicalize_symlink_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);
        sandbox.symlink("spam.txt", "link_to_spam.txt");

        let actual =
            canonicalize_with("link_to_spam.txt", dirs.test()).expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(windows))] // seems like Windows symlink requires existing file or dir
#[test]
fn canonicalize_symlink_loop_relative_to_should_fail() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        // sandbox.with_files(vec![EmptyFile("spam.txt")]);
        sandbox.symlink("spam.txt", "link_to_spam.txt");
        sandbox.symlink("link_to_spam.txt", "spam.txt");

        let actual = canonicalize_with("link_to_spam.txt", dirs.test());

        assert!(actual.is_err());
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn canonicalize_nested_symlink_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);
        sandbox.symlink("spam.txt", "link_to_spam.txt");
        sandbox.symlink("link_to_spam.txt", "link_to_link_to_spam.txt");

        let actual = canonicalize_with("link_to_link_to_spam.txt", dirs.test())
            .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn canonicalize_nested_symlink_within_symlink_dir_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("foo/bar/baz");
        sandbox.with_files(vec![EmptyFile("foo/bar/baz/spam.txt")]);
        sandbox.symlink("foo/bar/baz/spam.txt", "foo/bar/link_to_spam.txt");
        sandbox.symlink("foo/bar/link_to_spam.txt", "foo/link_to_link_to_spam.txt");
        sandbox.symlink("foo", "link_to_foo");

        let actual = canonicalize_with("link_to_foo/link_to_link_to_spam.txt", dirs.test())
            .expect("Failed to canonicalize");
        let mut expected = dirs.test().clone();
        expected.push("foo/bar/baz/spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn canonicalize_should_fail() {
    let path = Path::new("/foo/bar/baz"); // hopefully, this path does not exist

    let cwd = std::env::current_dir().expect("Could not get current directory");
    assert!(canonicalize_with(path, cwd).is_err());
}

#[test]
fn canonicalize_with_should_fail() {
    let relative_to = "/foo";
    let path = "bar/baz";

    assert!(canonicalize_with(path, relative_to).is_err());
}

#[cfg(windows)]
#[test]
fn canonicalize_unc() {
    // Ensure that canonicalizing UNC paths does not turn them verbatim.
    // Assumes the C drive exists and that the `localhost` UNC path works.
    let actual =
        nu_path::canonicalize_with(r"\\localhost\c$", ".").expect("failed to canonicalize");
    let expected = Path::new(r"\\localhost\c$");
    assert_eq!(actual, expected);
}
