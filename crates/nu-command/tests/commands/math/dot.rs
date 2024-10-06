use nu_test_support::nu;

#[test]
fn compute_dot_product_of_ints() {
    let actual = nu!("const RESULT = [1 2 3] | math dot [3 4 -5]; $RESULT");
    assert_eq!(actual.out, "-4");
}

#[test]
fn compute_dot_product_of_floats() {
    let actual = nu!("const RESULT = [1 2.0 3.5] | math dot [3.2 4.5 -5.6]; $RESULT");
    assert_eq!(actual.out, "-7.399999999999999");
}

#[test]
fn should_not_allow_vectors_with_different_dimensions() {
    let actual = nu!("[1 2 3] | math dot [3 4 5 6]");
    assert!(actual.err.contains("equal-length vectors"))
}

