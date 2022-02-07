use nu_test_support::{nu, pipeline};

// FIXME: jt: needs more work
#[ignore]
#[test]
fn generates_an_decimal() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        random decimal 42..43
        "#
    ));

    assert!(actual.out.contains("42") || actual.out.contains("43"));
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn generates_55() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        random decimal 55..55
        "#
    ));

    assert!(actual.out.contains("55"));
}

// FIXME: jt: needs more work
#[ignore]
#[test]
fn generates_0() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        random decimal ..<1
        "#
    ));

    assert!(actual.out.contains('0'));
}
