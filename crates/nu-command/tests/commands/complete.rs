use nu_test_support::nu;

#[test]
fn basic() {
    let actual = nu!(r#"
        (^echo a | complete) == {stdout: "a\n", exit_code: 0}
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn error() {
    let actual = nu!("do { not-found } | complete");

    assert!(actual.err.contains("executable was not found"));
}
