use nu_test_support::{nu, pipeline};

#[test]
fn add_overlay() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export def foo [] { "foo" } };
            overlay add spam;
            foo
        "#
    ));

    assert_eq!(actual.out, "foo");
}

#[test]
fn add_overlay_from_file() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            foo
        "#
    ));

    assert_eq!(actual.out, "foo");
}

#[test]
fn add_overlay_scoped() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export def foo [] { "foo" } };
            do { overlay add spam };
            foo
        "#
    ));

    assert!(actual.err.contains("External command"));
}

#[test]
fn remove_overlay() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export def foo [] { "foo" } };
            overlay add spam;
            overlay remove spam;
            foo
        "#
    ));

    assert!(actual.err.contains("External command"));
}

#[test]
fn remove_overlay_scoped() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export def foo [] { "foo" } };
            overlay add spam;
            do {
                overlay remove spam
            };
            foo
        "#
    ));

    assert_eq!(actual.out, "foo");
}
