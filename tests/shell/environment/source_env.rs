use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn source_env_eval_export_env() {
    Playground::setup("source_env_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { let-env FOO = 'foo' }
            "#,
        )]);

        let inp = &[r#"source-env spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn source_env_eval_export_env_hide() {
    Playground::setup("source_env_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { hide-env FOO }
            "#,
        )]);

        let inp = &[
            r#"let-env FOO = 'foo'"#,
            r#"source-env spam.nu"#,
            r#"$env.FOO"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert!(actual.err.contains("did you mean"));
    })
}

#[test]
fn source_env_do_cd() {
    Playground::setup("source_env_do_cd", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    cd test1/test2
                "#,
            )]);

        let inp = &[
            r#"source-env test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "test2");
    })
}

#[test]
fn source_env_do_cd_file_relative() {
    Playground::setup("source_env_do_cd_file_relative", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    cd ($env.FILE_PWD | path join '..')
                "#,
            )]);

        let inp = &[
            r#"source-env test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "test1");
    })
}

#[test]
fn source_env_dont_cd_overlay() {
    Playground::setup("source_env_dont_cd_overlay", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    overlay new spam
                    cd test1/test2
                    overlay hide spam
                "#,
            )]);

        let inp = &[
            r#"source-env test1/test2/spam.nu"#,
            r#"$env.PWD | path basename"#,
        ];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "source_env_dont_cd_overlay");
    })
}
