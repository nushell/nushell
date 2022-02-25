use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn returns_type_of_missing_file() {
    let actual = nu!(
        cwd: "tests", pipeline(
        r#"
            echo "spam.txt"
            | path type
        "#
    ));

    assert_eq!(actual.out, "");
}

#[test]
fn returns_type_of_existing_file() {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox
            .within("menu")
            .with_files(vec![EmptyFile("spam.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo "menu"
                | path type
            "#
        ));

        assert_eq!(actual.out, "dir");
    })
}

#[test]
fn returns_type_of_existing_directory() {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox
            .within("menu")
            .with_files(vec![EmptyFile("spam.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo "menu/spam.txt"
                | path type
            "#
        ));

        assert_eq!(actual.out, "file");
    })
}
