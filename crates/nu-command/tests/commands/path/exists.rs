use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn checks_if_existing_file_exists() {
    Playground::setup("path_exists_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("spam.txt")]);

        let actual = nu!(
            cwd: dirs.test(),
            "echo spam.txt | path exists"
        );

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn checks_if_missing_file_exists() {
    Playground::setup("path_exists_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "echo spam.txt | path exists"
        );

        assert_eq!(actual.out, "false");
    })
}

#[test]
fn checks_if_dot_exists() {
    Playground::setup("path_exists_3", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "echo '.' | path exists"
        );

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn checks_if_double_dot_exists() {
    Playground::setup("path_exists_4", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            "echo '..' | path exists"
        );

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn checks_tilde_relative_path_exists() {
    let actual = nu!(cwd: ".", "'~' | path exists");
    assert_eq!(actual.out, "true");
}
