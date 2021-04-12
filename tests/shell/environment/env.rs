use nu_test_support::nu;
use nu_test_support::playground::Playground;
use nu_test_support::{fs::Stub::FileWithContent, trusted::Trusted};

use serial_test::serial;

#[test]
fn passes_let_env_env_var_to_external_process() {
    let actual = nu!(cwd: ".", r#"
        let-env FOO = foo
        nu --testbin echo_env FOO
        "#);
    assert_eq!(actual.out, "foo");
}

#[test]
fn passes_with_env_env_var_to_external_process() {
    let actual = nu!(cwd: ".", r#"
        with-env [FOO foo] {nu --testbin echo_env FOO}
        "#);
    assert_eq!(actual.out, "foo");
}

#[test]
#[serial]
fn passes_env_from_local_cfg_to_external_process() {
    Playground::setup("autoenv_dir", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".nu-env",
            r#"[env]
            FOO = "foo"
            "#,
        )]);

        let actual = Trusted::in_path(&dirs, || {
            nu!(cwd: dirs.test(), r#"
                nu --testbin echo_env FOO
            "#)
        });

        assert_eq!(actual.out, "foo");
    })
}
