use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn gets_first_rows_by_amount() {
    Playground::setup("first_test_1", |dirs, sandbox| {
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
                | first 3
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn gets_all_rows_if_amount_higher_than_all_rows() {
    Playground::setup("first_test_2", |dirs, sandbox| {
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
                | first 99
                | length
            "#
        ));

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn gets_first_row_when_no_amount_given() {
    Playground::setup("first_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | first
                | length
            "#
        ));

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn gets_first_row_as_list_when_amount_given() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                [1, 2, 3]
                | first 1
                | describe
            "#
    ));

    assert_eq!(actual.out, "list<int>");
}
