use nu_test_support::nu;

#[test]
fn generates_an_integer() {
    let actual = nu!("random int 42..43");

    assert!(actual.out.contains("42") || actual.out.contains("43"));
}

#[test]
fn generates_55() {
    let actual = nu!("random int 55..55");

    assert!(actual.out.contains("55"));
}

#[test]
fn generates_0() {
    let actual = nu!("random int ..<1");

    assert!(actual.out.contains('0'));
}
