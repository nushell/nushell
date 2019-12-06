mod helpers;

use helpers::Playground;

#[test]
fn external_command() {
    let actual = nu!(
        cwd: "tests/fixtures",
        "echo 1"
    );

    assert!(actual.contains("1"));
}

#[test]
fn spawn_external_process_with_home_in_arguments() {
    Playground::setup("echo_tilde", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(),
            r#"
                sh -c "echo ~"
            "#
        );

        assert!(
            !actual.contains("~"),
            format!("'{}' should not contain ~", actual)
        );
    })
}

#[test]
fn spawn_external_process_with_tilde_in_arguments() {
    let actual = nu!(
        cwd: "tests/fixtures",
        r#"
            sh -c "echo 1~1"
        "#
    );

    assert_eq!(actual, "1~1");
}
