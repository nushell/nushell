use nu_test_support::{nu, pipeline};

#[test]
fn list_shells() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"g | get path | length "#
    ));

    assert_eq!(actual.out, "1");
}

#[test]
fn enter_shell() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"g 0"#
    ));

    assert!(actual.err.is_empty());
}

#[test]
fn enter_not_exist_shell() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"g 1"#
    ));

    assert!(actual.err.contains("Not found"));
}
