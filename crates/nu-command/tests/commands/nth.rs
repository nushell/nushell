use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn selects_a_row() {
    Playground::setup("select_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | sort-by name
                | select 0
                | get name
            "#
        ));

        assert_eq!(actual.out, "arepas.txt");
    });
}

#[test]
fn selects_many_rows() {
    Playground::setup("select_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("notes.txt"), EmptyFile("arepas.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | get name
                | select 1 0
                | length
            "#
        ));

        assert_eq!(actual.out, "2");
    });
}
