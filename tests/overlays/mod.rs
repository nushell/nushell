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
fn add_overlay_env() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export env FOO { "foo" } };
            overlay add spam;
            $env.FOO
        "#
    ));

    assert_eq!(actual.out, "foo");
}

#[test]
fn add_overlay_from_file_decl() {
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
fn add_overlay_from_file_alias() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            bar
        "#
    ));

    assert_eq!(actual.out, "bar");
}

#[test]
fn add_overlay_from_file_env() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            $env.BAZ
        "#
    ));

    assert_eq!(actual.out, "baz");
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

    assert!(!actual.err.is_empty())
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

    assert!(!actual.err.is_empty());
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

#[test]
fn remove_overlay_env() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export env FOO { "foo" } };
            overlay add spam;
            overlay remove spam;
            $env.FOO
        "#
    ));

    assert!(actual.err.contains("did you mean"));
}

#[test]
fn remove_overlay_scoped_env() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export env FOO { "foo" } };
            overlay add spam;
            do {
                overlay remove spam
            };
            $env.FOO
        "#
    ));

    assert_eq!(actual.out, "foo");
}

#[test]
fn list_last_overlay() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export def foo [] { "foo" } };
            overlay add spam;
            overlay list | last
        "#,
    ));

    assert_eq!(actual.out, "spam");
}

#[test]
fn list_overlay_scoped() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            module spam { export def foo [] { "foo" } };
            overlay add spam;
            do { overlay list | last }
        "#
    ));

    assert_eq!(actual.out, "spam");
}

#[test]
fn remove_overlay_discard_decl() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            def bagr [] { "bagr" };
            overlay remove spam;
            bagr
        "#
    ));

    assert!(!actual.err.is_empty());
}

#[test]
fn remove_overlay_discard_alias() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            alias bagr = "bagr";
            overlay remove spam;
            bagr
        "#
    ));

    assert!(!actual.err.is_empty());
}

#[test]
fn remove_overlay_discard_env() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            let-env BAGR = "bagr";
            overlay remove spam;
            $env.bagr
        "#
    ));

    assert!(actual.err.contains("did you mean"));
}

#[test]
fn preserve_overrides() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            def foo [] { "new-foo" };
            overlay remove spam;
            overlay add spam;
            foo
        "#
    ));

    assert_eq!(actual.out, "new-foo");
}

#[test]
fn reset_overrides() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            def foo [] { "new-foo" };
            overlay remove spam;
            overlay add samples/spam.nu;
            foo
        "#
    ));

    assert_eq!(actual.out, "foo");
}
