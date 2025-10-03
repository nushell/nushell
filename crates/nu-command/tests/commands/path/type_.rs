use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn returns_type_of_missing_file() {
    let actual = nu!(cwd: "tests", r#"
        echo "spam.txt"
        | path type
    "#);

    assert_eq!(actual.out, "");
}

#[test]
fn returns_type_of_existing_file() {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"
            echo "menu"
            | path type
        "#);

        assert_eq!(actual.out, "dir");
    })
}

#[test]
fn returns_type_of_existing_directory() {
    Playground::setup("path_expand_1", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"
            echo "menu/spam.txt"
            | path type
        "#);

        assert_eq!(actual.out, "file");

        let actual = nu!(r#"
            echo "~"
            | path type
        "#);

        assert_eq!(actual.out, "dir");
    })
}

#[test]
fn returns_type_of_existing_file_const() {
    Playground::setup("path_type_const", |dirs, sandbox| {
        sandbox.within("menu").with_files(&[EmptyFile("spam.txt")]);

        let actual = nu!(cwd: dirs.test(), r#"
            const ty = ("menu" | path type);
            $ty
        "#);

        assert_eq!(actual.out, "dir");
    })
}

#[test]
fn respects_cwd() {
    Playground::setup("path_type_respects_cwd", |dirs, sandbox| {
        sandbox.within("foo").with_files(&[EmptyFile("bar.txt")]);

        let actual = nu!(cwd: dirs.test(), "cd foo; 'bar.txt' | path type");

        assert_eq!(actual.out, "file");
    })
}
