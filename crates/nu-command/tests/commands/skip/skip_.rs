use nu_test_support::nu;

#[test]
fn binary_skip_will_raise_error() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open sample_data.ods --raw | skip 2"
    );

    assert!(actual.err.contains("only_supports_this_input_type"));
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!("1 | skip 2");

    assert!(actual.err.contains("command doesn't support"));
}
