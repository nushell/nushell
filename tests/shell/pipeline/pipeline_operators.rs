use nu_test_support::nu;

#[test]
fn and_operator1() {
    let actual = nu!(
        cwd: ".",
        r#"
            3 / 0 && 2 + 3
        "#
    );

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn and_operator2() {
    let actual = nu!(
        cwd: ".",
        r#"
            0 | 3 / $in && 2 + 3
        "#
    );

    assert!(actual.err.contains("division by zero"));
}

#[test]
fn or_operator1() {
    let actual = nu!(
        cwd: ".",
        r#"
            3 / 0 || 2 + 3
        "#
    );

    assert!(actual.out.contains('5'));
}

#[test]
fn or_operator2() {
    let actual = nu!(
        cwd: ".",
        r#"
            0 | 3 / $in || 2 + 3
        "#
    );

    assert!(actual.out.contains('5'));
}

#[test]
fn or_operator3() {
    // On success, don't run the next step
    let actual = nu!(
        cwd: ".",
        r#"
            0 || 2 + 3
        "#
    );

    assert_eq!(actual.out, "0");
}

#[test]
fn or_operator4() {
    let actual = nu!(
        cwd: ".",
        r#"
        1 / 0 || 2 / 0 || 10 + 9
        "#
    );

    assert!(actual.out.contains("19"));
}
