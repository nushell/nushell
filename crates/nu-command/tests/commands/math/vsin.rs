use nu_test_support::nu;

#[test]
fn compute_vcos_between_two_int_vectors() {
    let actual = nu!("[1 2 3] | math vsin [3 4 -5]");
    // Use "starts with" to avoid float inaccuracies
    assert!(actual.out.starts_with("0.988505365"))
}

#[test]
fn compute_vcos_between_two_float_vectors() {
    let actual = nu!("[1 2.0 3.55] | math vsin [3.2 4.7 -5]");
    // Use "starts with" to avoid float inaccuracies
    assert!(actual.out.starts_with("0.986771511"))
}

#[test]
fn should_not_accept_empty_pipeline() {
    let actual = nu!("[] | math vsin []");
    assert!(actual.err.contains("Unsupported input"));
}

#[test]
fn should_not_allow_vectors_with_different_dimensions() {
    let actual = nu!("[1 2 3] | math vsin [1 2]");
    assert!(actual.err.contains("Incorrect value"));
    assert!(actual.err.contains("equal-length vectors"))
}
