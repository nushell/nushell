use nu_test_support::prelude::*;

#[test]
fn generates_a_bool() -> Result {
    let code = "random bool | into string";
    let outcome: String = test().run(code)?;
    let is_boolean_output = outcome == "true" || outcome == "false";
    assert!(is_boolean_output);
    Ok(())
}
