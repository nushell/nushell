use nu_test_support::nu;

#[test]
fn generates_a_float() {
    let actual = nu!("random float 42..43");

    // Attention: this relies on the string output
    assert!(actual.out.starts_with("42") || actual.out.starts_with("43"));
    let actual = nu!("random float 42..43 | describe");

    assert_eq!(actual.out, "float")
}

#[test]
fn generates_55() {
    let actual = nu!("random float 55..55");

    assert!(actual.out.contains("55"));
}

#[test]
fn generates_0() {
    let actual = nu!("random float ..<1");

    assert!(actual.out.contains('0'));
}

#[test]
fn generate_inf() {
    let actual = nu!("random float 1.. | describe");
    assert_eq!(actual.out, "float");
}
