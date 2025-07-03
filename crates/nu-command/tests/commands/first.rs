use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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
fn gets_first_byte() {
    let actual = nu!("0x[aa bb cc] | first");

    assert_eq!(actual.out, "170");
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

    assert!(
        actual.err.contains("doesn't have flag `-1`"),
        "Expected unknown flag error, got: {}",
        actual.err
    );
}

#[test]
fn errors_on_empty_list_when_no_rows_given() {
    let actual = nu!("[] | first");

    assert!(actual.err.contains("index too large"));
}

#[test]
fn errors_on_extra_positional_argument() {
    let actual = nu!("[1, 2, 3] | first 2");
    assert!(actual.err.contains("extra positional argument"));
}

#[test]
fn errors_on_extra_positional_argument_from_command() {
    let actual = nu!("ls | first 1");
    assert!(actual.err.contains("extra positional argument"));
}
