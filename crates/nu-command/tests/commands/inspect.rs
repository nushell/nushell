use nu_test_support::nu;

#[test]
fn inspect_with_empty_pipeline() {
    let actual = nu!("inspect");
    assert!(actual.err.contains("no input value was piped in"));
}
