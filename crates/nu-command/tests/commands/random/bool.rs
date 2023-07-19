use nu_test_support::nu;

#[test]
fn generates_a_bool() {
    let actual = nu!("random bool");

    let output = actual.out;
    let is_boolean_output = output == "true" || output == "false";

    assert!(is_boolean_output);
}
