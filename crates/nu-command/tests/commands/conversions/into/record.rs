use nu_test_support::nu;

#[test]
fn doesnt_accept_mixed_type_list_as_input() {
    let actual = nu!("[{foo: bar} [quux baz]] | into record");
    assert!(!actual.status.success());
    assert!(actual.err.contains("type_mismatch"));
}
