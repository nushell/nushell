use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn gets_first_rows_by_amount() {
    Playground::setup("first_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(cwd: dirs.test(), "ls | first 3 | length");

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn gets_all_rows_if_amount_higher_than_all_rows() {
    Playground::setup("first_test_2", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("los.txt"),
            EmptyFile("tres.txt"),
            EmptyFile("amigos.txt"),
            EmptyFile("arepas.clu"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), "ls | first 99 | length");

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn gets_first_row_when_no_amount_given() {
    Playground::setup("first_test_3", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("caballeros.txt"), EmptyFile("arepas.clu")]);

        // FIXME: We should probably change first to return a one row table instead of a record here
        let actual = nu!(cwd: dirs.test(), "ls | first | values | length");

        assert_eq!(actual.out, "4");
    })
}

#[test]
fn gets_first_row_as_list_when_amount_given() {
    let actual = nu!("[1, 2, 3] | first 1 | describe");

    assert_eq!(actual.out, "list<int>");
}

#[test]
fn gets_first_bytes() {
    let actual = nu!("(0x[aa bb cc] | first 2) == 0x[aa bb]");

    assert_eq!(actual.out, "true");
}

#[test]
fn gets_first_byte() {
    let actual = nu!("0x[aa bb cc] | first");

    assert_eq!(actual.out, "170");
}

#[test]
fn gets_first_bytes_from_stream() {
    let actual = nu!("(1.. | each { 0x[aa bb cc] } | bytes collect | first 2) == 0x[aa bb]");

    assert_eq!(actual.out, "true");
}

#[test]
fn gets_first_byte_from_stream() {
    let actual = nu!("1.. | each { 0x[aa bb cc] } | bytes collect | first");

    assert_eq!(actual.out, "170");
}

#[test]
// covers a situation where `first` used to behave strangely on list<binary> input
fn works_with_binary_list() {
    let actual = nu!("([0x[01 11]] | first) == 0x[01 11]");

    assert_eq!(actual.out, "true");
}

#[test]
fn errors_on_negative_rows() {
    let actual = nu!("[1, 2, 3] | first -10");

    assert!(actual.err.contains("use a positive value"));
}

#[test]
fn errors_on_empty_list_when_no_rows_given() {
    let actual = nu!("[] | first");

    assert!(actual.err.contains("index too large"));
}

#[test]
fn gets_first_bytes_and_drops_content_type() {
    let actual = nu!(format!(
        "open {} | first 3 | metadata | get content_type? | describe",
        file!(),
    ));
    assert_eq!(actual.out, "nothing");
}
