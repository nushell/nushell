use nu_test_support::{nu, pipeline, playground::Playground};

#[test]
fn list_shells_1() {
    Playground::setup("list_shells_1", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; g| get active.2"#
        ));

        assert_eq!(actual.out, "true");
    })
}

#[test]
fn list_shells_2() {
    Playground::setup("list_shells_2", |dirs, sandbox| {
        sandbox.mkdir("foo").mkdir("bar");

        let actual = nu!(
            cwd: dirs.test(),
            pipeline(
            r#"enter foo; enter ../bar; shells| get active.2"#
        ));

        assert_eq!(actual.out, "true");
    })
}
