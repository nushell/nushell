use nu_test_support::{
    fs::Stub::{FileWithContent, FileWithContentToBeTrimmed},
    prelude::*,
};
use rstest::rstest;

#[test]
fn use_module_file_within_block() -> Result {
    Playground::setup("use_test_1", |dirs, playground| {
        let file = dirs.test().join("spam.nu");

        playground.with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            r#"
                export def foo [] {
                    echo "hello world"
                }
            "#,
        )]);

        let code = "
            def bar [] {
                use spam.nu foo;
                foo
            };
            bar
        ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("hello world")
    })
}

#[test]
fn use_keeps_doc_comments() -> Result {
    Playground::setup("use_doc_comments", |dirs, playground| {
        let file = dirs.test().join("spam.nu");

        playground.with_files(&[FileWithContent(
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

        let code = "
            use spam.nu foo;
            help foo
        ";

        let output: String = test().cwd(dirs.test()).run(code)?;
        assert_contains("this is my foo command", &output);
        assert_contains("this is an x parameter", &output);
        Ok(())
    })
}

#[test]
fn use_eval_export_env() -> Result {
    Playground::setup("use_eval_export_env", |dirs, playground| {
        playground.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                export-env { $env.FOO = 'foo' }
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu")?;
        tester.run("$env.FOO").expect_value_eq("foo")
    })
}

#[test]
fn use_eval_export_env_hide() -> Result {
    Playground::setup("use_eval_export_env", |dirs, playground| {
        playground.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                export-env { hide-env FOO }
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("$env.FOO = 'foo'")?;
        let () = tester.run("use spam.nu")?;

        let err = tester.run("$env.FOO").expect_shell_error()?;
        match err {
            ShellError::CantFindColumn { col_name, .. } => {
                assert_eq!(col_name, "FOO");
                Ok(())
            }
            err => Err(err.into()),
        }
    })
}

#[test]
fn use_do_cd() -> Result {
    Playground::setup("use_do_cd", |dirs, playground| {
        playground
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    export-env { cd test1/test2 }
                ",
            )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use test1/test2/spam.nu")?;
        tester
            .run("$env.PWD | path basename")
            .expect_value_eq("test2")
    })
}

#[test]
fn use_do_cd_file_relative() -> Result {
    Playground::setup("use_do_cd_file_relative", |dirs, playground| {
        playground
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    export-env { cd ($env.FILE_PWD | path join '..') }
                ",
            )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use test1/test2/spam.nu")?;
        tester
            .run("$env.PWD | path basename")
            .expect_value_eq("test1")
    })
}

#[test]
fn use_dont_cd_overlay() -> Result {
    Playground::setup("use_dont_cd_overlay", |dirs, playground| {
        playground
            .mkdir("test1/test2")
            .with_files(&[FileWithContentToBeTrimmed(
                "test1/test2/spam.nu",
                "
                    export-env {
                        overlay new spam
                        cd test1/test2
                        overlay hide spam
                    }
                ",
            )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use test1/test2/spam.nu")?;
        tester
            .run("$env.PWD | path basename")
            .expect_value_eq("use_dont_cd_overlay")
    })
}

#[test]
fn use_export_env_combined() -> Result {
    Playground::setup("use_is_scoped", |dirs, playground| {
        playground.with_files(&[FileWithContentToBeTrimmed(
            "spam.nu",
            "
                def foo [] { 'foo' }
                alias bar = foo
                export-env { $env.FOO = (bar) }
            ",
        )]);

        let mut tester = test().cwd(dirs.test());
        let () = tester.run("use spam.nu")?;
        tester.run("$env.FOO").expect_value_eq("foo")
    })
}

#[test]
fn use_module_creates_accurate_did_you_mean_1() -> Result {
    let code = r#"
        module spam {
            export def foo [] {
                "foo"
            }
        }
        
        use spam
        foo
    "#;

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::ExternalCommand { help, .. } => {
            assert_eq!(help, "Did you mean `spam foo`?");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn use_module_creates_accurate_did_you_mean_2() -> Result {
    let code = r#"
        module spam {
            export def foo [] {
                "foo"
            }
        }
        
        foo
    "#;

    let err = test().run(code).expect_shell_error()?;
    match err {
        ShellError::ExternalCommand { help, .. } => {
            assert_contains("A command with that name exists in module `spam`.", help);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[rstest]
#[case("use spam")]
#[case("use spam main")]
#[case("use spam [ main ]")]
#[case("use spam *")]
fn use_main(#[case] use_module: &str) -> Result {
    let mut tester = test();
    let () = tester.run(r#"module spam { export def main [] { "spam" } }"#)?;
    let () = tester.run(use_module)?;
    tester.run("spam").expect_value_eq("spam")
}

#[test]
fn use_main_def_env() -> Result {
    let mut tester = test();
    let () = tester.run(r#"module spam { export def --env main [] { $env.SPAM = "spam" } }"#)?;
    let () = tester.run("use spam")?;
    let () = tester.run("spam")?;
    tester.run("$env.SPAM").expect_value_eq("spam")
}

#[test]
fn use_main_def_known_external() -> Result {
    let mut tester = test().inherit_rust_toolchain_env();
    let () = tester.run("module cargo { export extern main [] }")?;
    let () = tester.run("use cargo")?;
    let outcome: String = tester.run("cargo --version")?;
    assert_contains("cargo", outcome);
    Ok(())
}

#[test]
fn use_main_not_exported() -> Result {
    let mut tester = test();
    let () = tester.run(r#"module unique-module-name { def main [] { "hi" } }"#)?;
    let () = tester.run("use unique-module-name")?;
    let err = tester.run("unique-module-name").expect_shell_error()?;
    assert!(matches!(err, ShellError::ExternalCommand { .. }));
    Ok(())
}

#[test]
fn use_sub_subname_error_if_not_from_submodule() -> Result {
    let code = "
        module spam {
            export def foo [] {}
            export def bar [] {}
        }

        use spam foo bar
    ";

    let err = test().run(code).expect_parse_error()?;
    match err {
        ParseError::WrongImportPattern(msg, ..) => {
            assert_contains("try `use <module> [<name1>, <name2>]`", msg);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn can_use_sub_subname_from_submodule() -> Result {
    let code = r#"
        module spam {
            export module foo {
                export def bar [] {
                    "bar"
                }
            }
        }

        use spam foo bar
        bar
    "#;

    test().run(code).expect_value_eq("bar")
}

#[test]
fn test_use_with_printing_file_pwd() -> Result {
    Playground::setup("use_with_printing_file_pwd", |dirs, playground| {
        let file = dirs.test().join("mod.nu");
        playground.with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            "
                export-env {
                    $env.CAPTURED_FILE_PWD = $env.FILE_PWD
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("use .; $env.CAPTURED_FILE_PWD")
            .expect_value_eq(dirs.test().to_string_lossy())
    })
}

#[test]
fn test_use_with_printing_current_file() -> Result {
    Playground::setup("use_with_printing_current_file", |dirs, playground| {
        let file = dirs.test().join("mod.nu");
        playground.with_files(&[FileWithContent(
            file.as_os_str().to_str().unwrap(),
            "
                export-env {
                    $env.CAPTURED_CURRENT_FILE = $env.CURRENT_FILE
                }
            ",
        )]);

        test()
            .cwd(dirs.test())
            .run("use .; $env.CAPTURED_CURRENT_FILE")
            .expect_value_eq(dirs.test().join("mod.nu").to_string_lossy())
    })
}

#[test]
fn report_errors_in_export_env() -> Result {
    let code = r#"
        module spam {
            export-env { error make -u {msg: "reported"} }
        }
        
        use spam
    "#;

    let err = test().run(code).expect_shell_error()?;
    assert_contains("reported", err.to_string());
    Ok(())
}
