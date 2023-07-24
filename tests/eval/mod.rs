use nu_test_support::nu;

#[test]
fn source_file_relative_to_file() {
    let actual = nu!("{x: 1, x: 2}");

    assert!(actual.err.contains("redefined"));
}
