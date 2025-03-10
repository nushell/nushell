use nu_test_support::nu;

#[test]
fn can_sqrt_numbers() {
    let actual = nu!("echo [0.25 2 4] | math sqrt | math sum");

    assert_eq!(actual.out, "3.914213562373095");
}

#[test]
fn can_sqrt_irrational() {
    let actual = nu!("echo 2 | math sqrt");

    assert_eq!(actual.out, "1.4142135623730951");
}

#[test]
fn can_sqrt_perfect_square() {
    let actual = nu!("echo 4 | math sqrt");

    assert_eq!(actual.out, "2");
}

#[test]
fn const_sqrt() {
    let actual = nu!("const SQRT = 4 | math sqrt; $SQRT");
    assert_eq!(actual.out, "2");
}

#[test]
fn cannot_sqrt_range() {
    let actual = nu!("0..5 | math sqrt");

    assert!(actual.err.contains("nu::parser::input_type_mismatch"));
}
