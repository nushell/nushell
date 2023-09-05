use nu_test_support::nu;

#[test]
fn source_file_relative_to_file() {
    let actual = nu!("{x: 1, x: 2}");

    assert!(actual.err.contains("redefined"));
}

#[test]
fn run_file_parse_error() {
    let actual = nu!(
        cwd: "tests/fixtures/eval",
        "nu script.nu"
    );

    assert!(actual.err.contains("unknown type"));
}
