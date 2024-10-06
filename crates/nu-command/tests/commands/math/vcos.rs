use nu_test_support::nu;

#[test]
fn compute_vcos_between_two_int_vectors() {
    let actual = nu!("[1 2 3] | math vcos [3 4 -5]");
    // Use "starts with" to avoid float inaccuracies
    assert!(actual.out.starts_with("-0.151185"))
}

#[test]
fn compute_vcos_between_two_float_vectors() {
    let actual = nu!("[1 2.0 3.55] | math vcos [3.2 4.7 -5]");
    // Use "starts with" to avoid float inaccuracies
    assert!(actual.out.starts_with("-0.162117"))
}

#[test]
fn should_not_accept_empty_pipeline() {
    let actual = nu!("[] | math vcos []");
    assert!(actual.err.contains("Unsupported input"));
}

#[test]
fn should_not_allow_vectors_with_different_dimensions() {
    let actual = nu!("[1 2 3] | math vcos [1 2]");
    assert!(actual.err.contains("Incorrect value"));
    assert!(actual.err.contains("equal-length vectors"))
}
