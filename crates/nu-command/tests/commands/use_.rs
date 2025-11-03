use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn use_module_file_within_block() {
    Playground::setup("use_test_1", |dirs, nu| {
        let file = dirs.test().join("spam.nu");

        nu.with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            r#"
                export def foo [] {
                    echo "hello world"
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            def bar [] {
                use spam.nu foo;
                foo
            };
            bar
        ");

        assert_eq!(actual.out, "hello world");
    })
}

#[test]
fn use_keeps_doc_comments() {
    Playground::setup("use_doc_comments", |dirs, nu| {
        let file = dirs.test().join("spam.nu");

        nu.with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            r#"
                # this is my foo command
                export def foo [
                    x:string # this is an x parameter
                ] {
                    echo "hello world"
                }
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), "
            use spam.nu foo;
            help foo
        ");

        assert!(actual.out.contains("this is my foo command"));
        assert!(actual.out.contains("this is an x parameter"));
    })
}

#[test]
fn use_eval_export_env() {
    Playground::setup("use_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { $env.FOO = 'foo' }
            "#,
        )]);

        let inp = &[r#"use spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), inp.join("; "));

        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn use_eval_export_env_hide() {
    Playground::setup("use_eval_export_env", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                export-env { hide-env FOO }
            "#,
        )]);

        let inp = &[r#"$env.FOO = 'foo'"#, r#"use spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), inp.join("; "));

        assert!(actual.err.contains("not_found"));
    })
}

#[test]
fn use_do_cd() {
    Playground::setup("use_do_cd", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env { cd test1/test2 }
                "#,
            )]);

        let inp = &[r#"use test1/test2/spam.nu"#, r#"$env.PWD | path basename"#];

        let actual = nu!(cwd: dirs.test(), inp.join("; "));

        assert_eq!(actual.out, "test2");
    })
}

#[test]
fn use_do_cd_file_relative() {
    Playground::setup("use_do_cd_file_relative", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                r#"
                    export-env { cd ($env.FILE_PWD | path join '..') }
                "#,
            )]);

        let inp = &[r#"use test1/test2/spam.nu"#, r#"$env.PWD | path basename"#];

        let actual = nu!(cwd: dirs.test(), inp.join("; "));

        assert_eq!(actual.out, "test1");
    })
}

#[test]
fn use_dont_cd_overlay() {
    Playground::setup("use_dont_cd_overlay", |dirs, sandbox| {
        sandbox
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
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

        let actual = nu!(cwd: dirs.test(), inp.join("; "));

        assert_eq!(actual.out, "use_dont_cd_overlay");
    })
}

#[test]
fn use_export_env_combined() {
    Playground::setup("use_is_scoped", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            r#"
                def foo [] { 'foo' }
                alias bar = foo
                export-env { $env.FOO = (bar) }
            "#,
        )]);

        let inp = &[r#"use spam.nu"#, r#"$env.FOO"#];

        let actual = nu!(cwd: dirs.test(), inp.join("; "));
        assert_eq!(actual.out, "foo");
    })
}

#[test]
fn use_module_creates_accurate_did_you_mean_1() {
    let actual = nu!(r#"
            module spam { export def foo [] { "foo" } }; use spam; foo
        "#);
    assert!(actual.err.contains("Did you mean `spam foo`"));
}

#[test]
fn use_module_creates_accurate_did_you_mean_2() {
    let actual = nu!(r#"
            module spam { export def foo [] { "foo" } }; foo
        "#);
    assert!(
        actual
            .err
            .contains("A command with that name exists in module `spam`")
    );
}

#[test]
fn use_main_1() {
    let inp = &[
        r#"module spam { export def main [] { "spam" } }"#,
        r#"use spam"#,
        r#"spam"#,
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn use_main_2() {
    let inp = &[
        r#"module spam { export def main [] { "spam" } }"#,
        r#"use spam main"#,
        r#"spam"#,
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn use_main_3() {
    let inp = &[
        r#"module spam { export def main [] { "spam" } }"#,
        r#"use spam [ main ]"#,
        r#"spam"#,
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn use_main_4() {
    let inp = &[
        r#"module spam { export def main [] { "spam" } }"#,
        r#"use spam *"#,
        r#"spam"#,
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn use_main_def_env() {
    let inp = &[
        r#"module spam { export def --env main [] { $env.SPAM = "spam" } }"#,
        r#"use spam"#,
        r#"spam"#,
        r#"$env.SPAM"#,
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn use_main_def_known_external() {
    // note: requires installed cargo
    let inp = &[
        r#"module cargo { export extern main [] }"#,
        r#"use cargo"#,
        r#"cargo --version"#,
    ];

    let actual = nu!(&inp.join("; "));

    assert!(actual.out.contains("cargo"));
}

#[test]
fn use_main_not_exported() {
    let inp = &[
        r#"module my-super-cool-and-unique-module-name { def main [] { "hi" } }"#,
        r#"use my-super-cool-and-unique-module-name"#,
        r#"my-super-cool-and-unique-module-name"#,
    ];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("external_command"));
}

#[test]
fn use_sub_subname_error_if_not_from_submodule() {
    let inp = r#"module spam { export def foo [] {}; export def bar [] {} }; use spam foo bar"#;
    let actual = nu!(inp);
    assert!(actual.err.contains("try `use <module> [<name1>, <name2>]`"))
}

#[test]
fn can_use_sub_subname_from_submodule() {
    let inp =
        r#"module spam { export module foo { export def bar [] {"bar"} } }; use spam foo bar; bar"#;
    let actual = nu!(inp);
    assert_eq!(actual.out, "bar")
}

#[test]
fn test_use_with_printing_file_pwd() {
    Playground::setup("use_with_printing_file_pwd", |dirs, nu| {
        let file = dirs.test().join("mod.nu");
        nu.with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            r#"
                export-env {
                    print $env.FILE_PWD
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "use ."
        );

        assert_eq!(actual.out, dirs.test().to_string_lossy());
    });
}

#[test]
fn test_use_with_printing_current_file() {
    Playground::setup("use_with_printing_current_file", |dirs, nu| {
        let file = dirs.test().join("mod.nu");
        nu.with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            r#"
                export-env {
                    print $env.CURRENT_FILE
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(),
            "use ."
        );

        assert_eq!(actual.out, dirs.test().join("mod.nu").to_string_lossy());
    });
}

#[test]
fn report_errors_in_export_env() {
    let actual = nu!(r#"
        module spam {
            export-env { error make -u {msg: "reported"} }
        }
        use spam
    "#);

    assert!(actual.err.contains("reported"));
}
