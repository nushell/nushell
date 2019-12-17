use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn selects_a_row() {
    Playground::setup("range_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("tests.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | sort-by name
                | range 0..0
                | get name
                | echo $it
            "#
        ));

        assert_eq!(actual, "notes.txt");
    });
}

#[test]
fn selects_some_rows() {
    Playground::setup("range_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("notes.txt"),
            EmptyFile("tests.txt"),
            EmptyFile("persons.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | range 1..2
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual, "2");
    });
}
