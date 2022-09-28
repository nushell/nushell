use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn gets_the_last_row() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | last 1 | get name.0 | str trim"
    );

    assert_eq!(actual.out, "utf16.ini");
}

#[test]
fn gets_last_rows_by_amount() {
    Playground::setup("last_test_1", |dirs, sandbox| {
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
                | last 3
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn gets_last_row_when_no_amount_given() {
    Playground::setup("last_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls
                | last
                | length
            "#
        ));

        assert_eq!(actual.out, "1");
    })
}

#[test]
fn requests_more_rows_than_table_has() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        date | last 50 | length
        "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn gets_last_row_as_list_when_amount_given() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                [1, 2, 3]
                | last 1
                | describe
            "#
    ));

    assert_eq!(actual.out, "list<int>");
}
