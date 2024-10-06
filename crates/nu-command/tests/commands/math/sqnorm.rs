use nu_test_support::nu;

#[test]
fn compute_magnitude_with_ints() {
    let actual = nu!("const RESULT = [1 2 3] | math sqnorm; $RESULT");
    assert_eq!(actual.out, "14");
}

#[test]
fn compute_magnitude_with_floats() {
    let actual = nu!("const RESULT = [1.0 2.55 3.9] | math sqnorm; $RESULT");
    assert_eq!(actual.out, "22.7125");
}

#[test]
fn should_not_accept_empty_pipeline() {
    let actual = nu!("[] | math sqnorm");
    assert!(actual.err.contains("Unsupported input"));
}
