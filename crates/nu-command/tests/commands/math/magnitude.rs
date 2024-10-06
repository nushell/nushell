use nu_test_support::nu;

#[test]
fn compute_magnitude_with_ints() {
    let actual = nu!("const RESULT = [1 2 3] | math magnitude; $RESULT");
    // Use "starts with" to avoid float inaccuracies
    assert!(actual.out.starts_with("3.74165738677"));
}

#[test]
fn compute_magnitude_with_floats() {
    let actual = nu!("const RESULT = [1.0 2.55 3.9] | math magnitude; $RESULT");
    // Use "starts with" to avoid float inaccuracies
    assert!(actual.out.starts_with("4.76576331766"));
}

#[test]
fn should_not_accept_empty_pipeline() {
    let actual = nu!("[] | math magnitude");
    assert!(actual.err.contains("Unsupported input"));
}
