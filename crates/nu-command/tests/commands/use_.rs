use nu_test_support::fs::AbsolutePath;
use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::nu;
use nu_test_support::pipeline;
use nu_test_support::playground::Playground;

#[test]
fn use_module_file_within_block() {
    Playground::setup("use_test_1", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("spam.nu"));

        nu.with_files(vec![FileWithContent(
            &file.to_string(),
            r#"
                export def foo [] {
                    echo "hello world"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
                r#"
                    def bar [] {
                        use spam.nu foo;
                        foo
                    };
                    bar
                "#
            )
        );

        assert_eq!(actual.out, "hello world");
    })
}

#[test]
fn use_keeps_doc_comments() {
    Playground::setup("use_doc_comments", |dirs, nu| {
        let file = AbsolutePath::new(dirs.test().join("spam.nu"));

        nu.with_files(vec![FileWithContent(
            &file.to_string(),
            r#"
                # this is my foo command
                export def foo [
                    x:string # this is an x parameter
                ] {
                    echo "hello world"
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
                r#"
                    use spam.nu foo;
                    help foo
                "#
            )
        );

        assert!(actual.out.contains("this is my foo command"));
        assert!(actual.out.contains("this is an x parameter"));
    })
}

#[test]
fn use_eval_export_env() {
    Playground::setup("use_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { let-env FOO = 'foo' }
            "#,
        )]);

        let inp = &[r#"use spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn use_eval_export_env_hide() {
    Playground::setup("use_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { hide-env FOO }
            "#,
        )]);

        let inp = &[r#"let-env FOO = 'foo'"#, r#"use spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert!(actual.err.contains("cannot find column"));
    })
}

#[test]
fn use_do_cd() {
    Playground::setup("use_do_cd", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env { cd test1/test2 }
                "#,
            )]);

        let inp = &[r#"use test1/test2/spam.nu"#, r#"$env.PWD | path basename"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "test2");
    })
}

#[test]
fn use_do_cd_file_relative() {
    Playground::setup("use_do_cd_file_relative", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env { cd ($env.FILE_PWD | path join '..') }
                "#,
            )]);

        let inp = &[r#"use test1/test2/spam.nu"#, r#"$env.PWD | path basename"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "test1");
    })
}

#[test]
fn use_dont_cd_overlay() {
    Playground::setup("use_dont_cd_overlay", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(vec![FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env {
                        overlay new spam
                        cd test1/test2
                        overlay hide spam
                    }
                "#,
            )]);

        let inp = &[r#"use test1/test2/spam.nu"#, r#"$env.PWD | path basename"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));

        assert_eq!(actual.out, "use_dont_cd_overlay");
    })
}

#[test]
fn use_export_env_combined() {
    Playground::setup("use_is_scoped", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                alias bar = foo
                export-env { let-env FOO = bar }
                def foo [] { 'foo' }
            "#,
        )]);

        let inp = &[r#"use spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), pipeline(&inp.join("; ")));
        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn use_module_creates_accurate_did_you_mean() {
    let actual = nu!(
    cwd: ".", pipeline(
        r#"
                module spam { export def foo [] { "foo" } }; use spam; foo
            "#
        )
    );
    assert!(actual.err.contains(
        "command 'foo' was not found but it exists in module 'spam'; try using `spam foo`"
    ));
}
