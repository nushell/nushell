use nu_test_support::nu;

#[test]
fn runs_successfully() {
    let actual = nu!("debug info");
    assert_eq!(actual.err, "");
}
