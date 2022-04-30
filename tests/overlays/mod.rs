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
fn add_overlay_from_file1() {
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
fn add_overlay_from_file2() {
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
fn add_overlay_from_file3() {
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

#[ignore]
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

#[ignore]
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
fn remove_overlay_merge_decl() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            def bagr [] { "bagr" };
            overlay remove spam;
            bagr
        "#
    ));

    assert_eq!(actual.out, "bagr");
}

#[test]
fn remove_overlay_merge_alias() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            alias bagr = "bagr";
            overlay remove spam;
            bagr
        "#
    ));

    assert_eq!(actual.out, "bagr");
}

#[ignore]
#[test]
fn remove_overlay_merge_env() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            let-env bagr = "bagr";
            overlay remove spam;
            $env.bagr
        "#
    ));

    assert_eq!(actual.out, "bagr");
}

#[test]
fn remove_overlay_discard_decl() {
    let actual = nu!(
        cwd: "tests/overlays", pipeline(
        r#"
            overlay add samples/spam.nu;
            def bagr [] { "bagr" };
            overlay remove -d spam;
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
            overlay remove -d spam;
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
            overlay remove -d spam;
            $env.bagr
        "#
    ));

    assert!(actual.err.contains("did you mean"));
}
