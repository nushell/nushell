use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn gets_all_rows_by_every_zero() {
    Playground::setup("every_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | every 0
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn gets_all_rows_by_every_one() {
    Playground::setup("every_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | every 1
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn gets_first_row_by_every_too_much() {
    Playground::setup("every_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | every 999
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn gets_every_third_row() {
    Playground::setup("every_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("quatro.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | every 3
                | count
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "2");
    })
}
