use nu_path::absolute_with;
use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use pretty_assertions::assert_eq;
use std::path::Path;

#[test]
fn absolute_path() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut spam = dirs.test().to_owned();
        spam.push("spam.txt");

        let cwd = std::env::current_dir().expect("Could not get current directory");
        let actual = absolute_with(spam, cwd).expect("Failed to make absolute");

        assert!(actual.ends_with("spam.txt"));
    });
}

#[test]
fn absolute_unicode_path() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut spam = dirs.test().to_owned();
        spam.push("🚒.txt");

        let cwd = std::env::current_dir().expect("Could not get current directory");

        let actual = absolute_with(spam, cwd).expect("Failed to make absolute");

        assert!(actual.ends_with("🚒.txt"));
    });
}

#[ignore]
#[test]
fn absolute_non_utf8_path() {
    // TODO
}

#[test]
fn absolute_path_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = absolute_with("spam.txt", dirs.test()).expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_unicode_path_relative_to_unicode_path_with_spaces() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut relative_to = dirs.test().to_owned();
        relative_to.push("e-$ èрт🚒♞中片-j");

        let actual = absolute_with("🚒.txt", relative_to).expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();
        expected.push("e-$ èрт🚒♞中片-j/🚒.txt");

        assert_eq!(actual, expected);
    });
}

#[ignore]
#[test]
fn absolute_non_utf8_path_relative_to_non_utf8_path_with_spaces() {
    // TODO
}

#[test]
fn absolute_absolute_path_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut absolute_path = dirs.test().to_owned();
        absolute_path.push("spam.txt");

        let actual = absolute_with(&absolute_path, "non/existent/directory")
            .expect("Failed to make absolute");
        let expected = absolute_path;

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_dot() {
    let expected = std::env::current_dir().expect("Could not get current directory");

    let actual = absolute_with(".", expected.as_path()).expect("Failed to make absolute");

    assert_eq!(actual, expected);
}

#[test]
fn absolute_many_dots() {
    let expected = std::env::current_dir().expect("Could not get current directory");

    let actual = absolute_with("././/.//////./././//.///", expected.as_path())
        .expect("Failed to make absolute");

    assert_eq!(actual, expected);
}

#[test]
fn absolute_path_with_dot_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = absolute_with("./spam.txt", dirs.test()).expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_path_with_many_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = absolute_with("././/.//////./././//.////spam.txt", dirs.test())
            .expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();
        expected.push("spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_double_dot() {
    let cwd = std::env::current_dir().expect("Could not get current directory");
    let actual = absolute_with("..", &cwd).expect("Failed to make absolute");

    // On Windows .. components are resolved. On Unix they are not.
    #[cfg(windows)]
    let expected = cwd
        .parent()
        .expect("Could not get parent of current directory");
    #[cfg(not(windows))]
    let expected = cwd.join("..");

    assert_eq!(actual, expected);
}

#[test]
fn absolute_path_with_double_dot_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual =
            absolute_with("foo/../spam.txt", dirs.test()).expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("spam.txt");
        #[cfg(not(windows))]
        expected.push("foo/../spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_path_with_many_double_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = absolute_with("foo/bar/baz/../../../spam.txt", dirs.test())
            .expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("spam.txt");
        #[cfg(not(windows))]
        expected.push("foo/bar/baz/../../../spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_ndots2() {
    // This test will fail if you have the nushell repo on the root partition
    // So, let's start in a nested folder before trying to absolute_with "..."
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.mkdir("aaa/bbb/ccc");
        let output = nu!( cwd: dirs.root(), "cd nu_path_test_1/aaa/bbb/ccc; $env.PWD");
        let cwd = Path::new(&output.out);

        let actual = absolute_with("...", cwd).expect("Failed to make absolute");
        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        let expected = cwd
            .parent()
            .expect("Could not get parent of current directory")
            .parent()
            .expect("Could not get parent of a parent of current directory");
        #[cfg(not(windows))]
        let expected = cwd.join("../..");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_path_with_3_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual =
            absolute_with("foo/bar/.../spam.txt", dirs.test()).expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("spam.txt");
        #[cfg(not(windows))]
        expected.push("foo/bar/../../spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_path_with_many_3_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = absolute_with(
            "foo/bar/baz/eggs/sausage/bacon/.../.../.../spam.txt",
            dirs.test(),
        )
        .expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("spam.txt");
        #[cfg(not(windows))]
        expected.push("foo/bar/baz/eggs/sausage/bacon/../../../../../../spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_path_with_4_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = absolute_with("foo/bar/baz/..../spam.txt", dirs.test())
            .expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("spam.txt");
        #[cfg(not(windows))]
        expected.push("foo/bar/baz/../../../spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_path_with_many_4_ndots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let actual = absolute_with(
            "foo/bar/baz/eggs/sausage/bacon/..../..../spam.txt",
            dirs.test(),
        )
        .expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("spam.txt");
        #[cfg(not(windows))]
        expected.push("foo/bar/baz/eggs/sausage/bacon/../../../../../../spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_path_with_way_too_many_dots_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut relative_to = dirs.test().to_owned();
        relative_to.push("foo/bar/baz/eggs/sausage/bacon/vikings");

        let actual = absolute_with("././..////././...///././.....///spam.txt", relative_to)
            .expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("spam.txt");
        #[cfg(not(windows))]
        expected.push("foo/bar/baz/eggs/sausage/bacon/vikings/../../../../../../../spam.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_unicode_path_with_way_too_many_dots_relative_to_unicode_path_with_spaces() {
    Playground::setup("nu_path_test_1", |dirs, _| {
        let mut relative_to = dirs.test().to_owned();
        relative_to.push("foo/áčěéí  +šř=é/baz/eggs/e-$ èрт🚒♞中片-j/bacon/öäöä öäöä");

        let actual = absolute_with("././..////././...///././.....///🚒.txt", relative_to)
            .expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();

        // On Windows .. components are resolved. On Unix they are not.
        #[cfg(windows)]
        expected.push("🚒.txt");
        #[cfg(not(windows))]
        expected.push("foo/áčěéí  +šř=é/baz/eggs/e-$ èрт🚒♞中片-j/bacon/öäöä öäöä/../../../../../../../🚒.txt");

        assert_eq!(actual, expected);
    });
}

#[test]
fn absolute_tilde() {
    let tilde_path = "~";

    let cwd = std::env::current_dir().expect("Could not get current directory");
    let actual = absolute_with(tilde_path, cwd).expect("Failed to make absolute");

    assert!(actual.is_absolute());
    assert!(!actual.starts_with("~"));
}

#[test]
fn absolute_tilde_relative_to() {
    let tilde_path = "~";

    let actual = absolute_with(tilde_path, "non/existent/path").expect("Failed to make absolute");

    assert!(actual.is_absolute());
    assert!(!actual.starts_with("~"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn absolute_symlink() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.txt")]);
        sandbox.symlink("spam.txt", "link_to_spam.txt");

        let mut symlink_path = dirs.test().to_owned();
        symlink_path.push("link_to_spam.txt");

        let cwd = std::env::current_dir().expect("Could not get current directory");
        let actual = absolute_with(symlink_path, cwd).expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();
        expected.push("link_to_spam.txt");

        assert_eq!(actual, expected);
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn absolute_symlink_relative_to() {
    Playground::setup("nu_path_test_1", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("spam.txt")]);
        sandbox.symlink("spam.txt", "link_to_spam.txt");

        let actual =
            absolute_with("link_to_spam.txt", dirs.test()).expect("Failed to make absolute");
        let mut expected = dirs.test().to_owned();
        expected.push("link_to_spam.txt");

        assert_eq!(actual, expected);
    });
}
