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

<<<<<<< HEAD
=======
// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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

<<<<<<< HEAD
        assert_eq!(actual.out, "Dir");
    })
}

=======
        assert_eq!(actual.out, "dir");
    })
}

// FIXME: jt: needs more work
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
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

<<<<<<< HEAD
        assert_eq!(actual.out, "File");
=======
        assert_eq!(actual.out, "file");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    })
}
