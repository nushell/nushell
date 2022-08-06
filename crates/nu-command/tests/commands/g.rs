use nu_test_support::{nu, pipeline, playground::Playground};

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

#[test]
fn switch_to_last_used_shell_1() {
    Playground::setup("switch_last_used_shell_1", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; g 0; g -;g | get active.2"#
        ));

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn switch_to_last_used_shell_2() {
    Playground::setup("switch_last_used_shell_2", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; n; g -;g | get active.2"#
        ));

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn switch_to_last_used_shell_3() {
    Playground::setup("switch_last_used_shell_3", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; p; g -;g | get active.2"#
        ));

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn switch_to_last_used_shell_4() {
    Playground::setup("switch_last_used_shell_4", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; g 2; exit; g -;g | get active.0"#
        ));

        assert_eq!(actual.out, "true");
    })
}
