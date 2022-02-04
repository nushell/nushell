use nu_test_support::{nu, pipeline};

#[test]
fn generates_a_bool() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        random bool
        "#
    ));

    let output = actual.out;
    let is_boolean_output = output == "true" || output == "false";

    assert!(is_boolean_output);
}
