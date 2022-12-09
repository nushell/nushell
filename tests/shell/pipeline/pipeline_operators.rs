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

    assert_eq!(actual.out, "5");
}

#[test]
fn or_operator2() {
    let actual = nu!(
        cwd: ".",
        r#"
            0 | 3 / $in || 2 + 3
        "#
    );

    assert_eq!(actual.out, "5");
}
