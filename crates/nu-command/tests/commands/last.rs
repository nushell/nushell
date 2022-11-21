use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn gets_the_last_row() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | sort-by name | last | get name"
    );

    assert_eq!(actual.out, "utf16.ini");
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
// TODO: maybe someday we should support this by returning the last char?
fn fails_on_string() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                "foo bar" | last
            "#
    ));

    assert!(actual.err.contains("unsupported_input"));
}

#[test]
fn fails_on_int() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                3 | last
            "#
    ));

    assert!(actual.err.contains("unsupported_input"));
}
