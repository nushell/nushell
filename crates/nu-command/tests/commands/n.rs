use nu_test_support::{nu, pipeline, playground::Playground};

#[test]
fn switch_to_next_shell_1() {
    Playground::setup("switch_to_next_shell_1", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; n; g | get active.0"#
        ));

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn switch_to_next_shell_2() {
    Playground::setup("switch_to_next_shell_2", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; n; n; g | get active.1"#
        ));

        assert_eq!(actual.out, "true");
    })
}
