use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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

        let actual = nu!(cwd: dirs.test(), "ls | last 3 | length");

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn gets_last_row_when_no_amount_given() {
    Playground::setup("last_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        // FIXME: We should probably change last to return a one row table instead of a record here
        let actual = nu!(cwd: dirs.test(), "ls | last | values | length");

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn requests_more_rows_than_table_has() {
    let actual = nu!("[date] | last 50 | length");

    assert_eq!(actual.out, "1");
}

#[test]
fn gets_last_row_as_list_when_amount_given() {
    let actual = nu!("[1, 2, 3] | last 1 | describe");

    assert_eq!(actual.out, "list<int>");
}

#[test]
fn gets_last_bytes() {
    let actual = nu!("(0x[aa bb cc] | last 2) == 0x[bb cc]");

    assert_eq!(actual.out, "true");
}

#[test]
fn gets_last_byte() {
    let actual = nu!("0x[aa bb cc] | last");

    assert_eq!(actual.out, "204");
}

#[test]
fn last_errors_on_negative_index() {
    let actual = nu!("[1, 2, 3] | last -2");

    assert!(actual.err.contains("use a positive value"));
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | last");

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn errors_on_empty_list_when_no_rows_given() {
    let actual = nu!("[] | last");

    assert!(actual.err.contains("index too large"));
}
