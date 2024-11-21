use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn length_columns_in_cal_table() {
    let actual = nu!("cal --as-table | columns | length");

    assert_eq!(actual.out, "7");
}

#[test]
fn length_columns_no_rows() {
    let actual = nu!("echo [] | length");

    assert_eq!(actual.out, "0");
}

#[test]
fn length_fails_on_echo_record() {
    let actual = nu!("echo {a:1 b:2} | length");

    assert!(actual.err.contains("only_supports_this_input_type"));
}

#[test]
fn length_byte_stream() {
    Playground::setup("length_bytes", |dirs, sandbox| {
        sandbox.mkdir("length_bytes");
        sandbox.with_files(&[FileWithContent("data.txt", "😀")]);

        let actual = nu!(cwd: dirs.test(), "open data.txt | length");
        assert_eq!(actual.out, "4");
    });
}
