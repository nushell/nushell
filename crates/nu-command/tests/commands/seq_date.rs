use nu_test_support::nu;

#[test]
fn fails_when_output_format_contains_time() {
    let actual = nu!("seq date --output-format '%H-%M-%S'");

    assert!(actual.err.contains("Invalid output format"));
}
