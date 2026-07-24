use std::{fs, io};

use nu_test_support::{fs::Stub::EmptyFile, prelude::*};
use rstest::rstest;
use rstest_reuse::{apply, template};

// Windows .ps1 tests run with NuTester's intentionally minimal environment.
// Keep the environment narrow but sufficient:
// PATHEXT is loaded by NuTester so .ps1 is treated as executable, PATH is inherited where
// PowerShell must be resolved, and SystemRoot is inherited for Windows/PowerShell process startup
// in an otherwise stripped environment.

// Template for run-external test to ensure tests work when calling
// the binary directly, using the caret operator, and when using
// the run-external command
#[template]
#[rstest]
#[case::bare("")]
#[case::caret("^")]
#[case::run_external("run-external ")]
fn run_external_prefixes(#[case] prefix: &str) {}

// Template for tests that only cover direct binary calls and the caret operator.
// Use this when `run-external` would change argument parsing semantics.
#[template]
#[rstest]
#[case::bare("")]
#[case::caret("^")]
fn direct_external_prefixes(#[case] prefix: &str) {}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn better_empty_redirection(prefix: &str) -> Result {
    let code = format!("ls | each {{ |it| {prefix}cococo $it.name }} | ignore");

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(())
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn explicit_glob(prefix: &str) -> Result {
    Playground::setup("external with explicit glob", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual: String = test()
            .cwd(dirs.test())
            .run(format!("{prefix}cococo ('*.txt' | into glob)"))?;

        assert_contains("D&D_volume_1.txt", &actual);
        assert_contains("D&D_volume_2.txt", actual);
        Ok(())
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn bare_word_expand_path_glob(prefix: &str) -> Result {
    Playground::setup("bare word should do the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual: String = test()
            .cwd(dirs.test())
            .run(format!("{prefix}cococo *.txt"))?;

        assert_contains("D&D_volume_1.txt", &actual);
        assert_contains("D&D_volume_2.txt", actual);
        Ok(())
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn backtick_expand_path_glob(prefix: &str) -> Result {
    Playground::setup("backtick should do the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        let actual: String = test()
            .cwd(dirs.test())
            .run(format!("{prefix}cococo `*.txt`"))?;

        assert_contains("D&D_volume_1.txt", &actual);
        assert_contains("D&D_volume_2.txt", actual);
        Ok(())
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn single_quote_does_not_expand_path_glob(prefix: &str) -> Result {
    Playground::setup("single quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        test()
            .cwd(dirs.test())
            .run(format!("{prefix}cococo '*.txt'"))
            .expect_value_eq("*.txt")
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn double_quote_does_not_expand_path_glob(prefix: &str) -> Result {
    Playground::setup("double quote do not run the expansion", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("D&D_volume_1.txt"),
            EmptyFile("D&D_volume_2.txt"),
            EmptyFile("foo.sh"),
        ]);

        test()
            .cwd(dirs.test())
            .run(format!(r#"{prefix}cococo "*.txt""#))
            .expect_value_eq("*.txt")
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_FAIL)]
fn failed_command_with_semicolon_will_not_execute_following_cmds(prefix: &str) -> Result {
    Playground::setup("external failed command with semicolon", |dirs, _| {
        let code = format!("try {{ {prefix}fail; echo done }} catch {{ 'stopped' }}");

        test().cwd(dirs.test()).run(code).expect_value_eq("stopped")
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_args_with_quoted(prefix: &str) -> Result {
    Playground::setup("external args with quoted", |dirs, _| {
        test()
            .cwd(dirs.test())
            .run(format!(r#"{prefix}cococo "foo=bar 'hi'""#))
            .expect_value_eq("foo=bar 'hi'")
    })
}

#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_with_option_like_embedded_quotes(prefix: &str) -> Result {
    Playground::setup(
        "external arg with option like embedded quotes",
        |dirs, _| {
            test()
                .cwd(dirs.test())
                .run(format!("{prefix}cococo -- --foo='bar' -foo='bar'"))
                .expect_value_eq("--foo=bar -foo=bar")
        },
    )
}

// FIXME: parser complains about invalid characters after single quote
#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_with_non_option_like_embedded_quotes(prefix: &str) -> Result {
    Playground::setup(
        "external arg with non option like embedded quotes",
        |dirs, _| {
            test()
                .cwd(dirs.test())
                .run(format!("{prefix}cococo foo='bar' 'foo'=bar"))
                .expect_value_eq("foo=bar foo=bar")
        },
    )
}

// FIXME: parser bug prevents expressions from appearing within GlobPattern substrings
#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_with_string_interpolation(prefix: &str) -> Result {
    Playground::setup("external arg with string interpolation", |dirs, _| {
        let code = format!(r#"{prefix}cococo foo=(2 + 2) $"foo=(2 + 2)" foo=$"(2 + 2)""#);

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("foo=4 foo=4 foo=4")
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_NONU)]
fn external_arg_with_variable_name(prefix: &str) -> Result {
    Playground::setup("external arg with variable name", |dirs, _| {
        let code = format!(
            r#"
                let dump_command = "PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'"
                {prefix}nonu $dump_command
            "#
        );

        test().cwd(dirs.test()).run(code).expect_value_eq(
            "PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'",
        )
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_command_escape_args(prefix: &str) -> Result {
    Playground::setup("external command escape args", |dirs, _| {
        test()
            .cwd(dirs.test())
            .run(format!(r#"{prefix}cococo "\"abcd""#))
            .expect_value_eq(r#""abcd"#)
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_command_ndots_args(prefix: &str) -> Result {
    let code = format!(
        "{prefix}cococo foo/. foo/.. foo/... foo/./bar foo/../bar foo/.../bar ./bar ../bar .../bar"
    );

    test().run(code).expect_value_eq(cfg_select! {
        windows => {
            // Windows is a bit weird right now, where if ndots has to fix something it's going to
            // change everything to backslashes too. Would be good to fix that
            r"foo/. foo/.. foo\..\.. foo/./bar foo/../bar foo\..\..\bar ./bar ../bar ..\..\bar"
        }
        _ => {
            "foo/. foo/.. foo/../.. foo/./bar foo/../bar foo/../../bar ./bar ../bar ../../bar"
        }
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_command_ndots_leading_dot_slash(prefix: &str) -> Result {
    // Don't expand ndots with a leading `./`
    test()
        .run(format!("{prefix}cococo ./... ./...."))
        .expect_value_eq("./... ./....")
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_command_url_args(prefix: &str) -> Result {
    // If ndots is not handled correctly, we can lose the double forward slashes that are needed
    // here
    let code = format!("{prefix}cococo http://example.com http://example.com/.../foo //foo");

    test()
        .run(code)
        .expect_value_eq("http://example.com http://example.com/.../foo //foo")
}

#[apply(run_external_prefixes)]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "only runs on Linux, where controlling the HOME var is reliable"
)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_command_expand_tilde(prefix: &str) -> Result {
    Playground::setup("external command expand tilde", |dirs, _| {
        // Make a copy of the testbin that can be found through tilde expansion.
        let mut src = fs::File::open(TESTBIN_COCOCO.path())?;
        let testbin_path = dirs.test().join("test_cococo");
        let mut dst = fs::File::create_new(&testbin_path)?;
        io::copy(&mut src, &mut dst)?;

        dst.set_permissions(src.metadata()?.permissions())?;
        drop(dst);
        drop(src);

        test()
            .env("HOME", dirs.test().to_string_lossy())
            .run(format!("{prefix}~/test_cococo hello"))
            .expect_value_eq("hello")
    })
}

// FIXME: parser bug prevents expressions from appearing within GlobPattern substrings
#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_expand_tilde(prefix: &str) -> Result {
    Playground::setup("external arg expand tilde", |dirs, _| {
        let home = dirs::home_dir().expect("failed to find home dir");
        test()
            .cwd(dirs.test())
            .run(format!("{prefix}cococo ~/foo ~/(2 + 2)"))
            .expect_value_eq(format!(
                "{} {}",
                home.join("foo").display(),
                home.join("4").display()
            ))
    })
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_NONU)]
fn external_command_not_expand_tilde_with_quotes(prefix: &str) -> Result {
    Playground::setup(
        "external command not expand tilde with quotes",
        |dirs, _| {
            test()
                .cwd(dirs.test())
                .run(format!(r#"{prefix}nonu "~""#))
                .expect_value_eq("~")
        },
    )
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_NONU)]
fn external_command_expand_tilde_with_back_quotes(prefix: &str) -> Result {
    Playground::setup(
        "external command expand tilde with back quotes",
        |dirs, _| {
            let actual: String = test().cwd(dirs.test()).run(format!("{prefix}nonu `~`"))?;
            assert_contains_not("~", actual);
            Ok(())
        },
    )
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_INPUT_BYTES_LENGTH)]
fn external_command_receives_raw_binary_data(prefix: &str) -> Result {
    Playground::setup("external command receives raw binary data", |dirs, _| {
        test()
            .cwd(dirs.test())
            .run(format!("0x[deadbeef] | {prefix}input_bytes_length"))
            .expect_value_eq("4")
    })
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_cmd_files(prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run a Windows cmd file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.cmd",
            "
                @echo off
                echo Hello World
            ",
        )]);

        let actual: String = test().cwd(dirs.test()).run(format!("{prefix}foo.cmd"))?;
        assert_contains("Hello World", actual);
        Ok(())
    })
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_batch_files(prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run a Windows batch file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.bat",
            "
                @echo off
                echo Hello World
            ",
        )]);

        let actual: String = test().cwd(dirs.test()).run(format!("{prefix}foo.bat"))?;
        assert_contains("Hello World", actual);
        Ok(())
    })
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_batch_files_without_cmd_extension(prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup(
        "run a Windows cmd file without specifying the extension",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                "foo.cmd",
                "
                @echo off
                echo Hello World
            ",
            )]);

            let actual: String = test().cwd(dirs.test()).run(format!("{prefix}foo"))?;
            assert_contains("Hello World", actual);
            Ok(())
        },
    )
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_batch_files_without_bat_extension(prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup(
        "run a Windows batch file without specifying the extension",
        |dirs, sandbox| {
            sandbox.with_files(&[FileWithContent(
                "foo.bat",
                "
                @echo off
                echo Hello World
            ",
            )]);

            let actual: String = test().cwd(dirs.test()).run(format!("{prefix}foo"))?;
            assert_contains("Hello World", actual);
            Ok(())
        },
    )
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn quotes_trimmed_when_shelling_out(prefix: &str) -> Result {
    // Regression test for a bug where quotes around string args weren't trimmed before shelling out to cmd.exe.
    test()
        .run(format!(r#"{prefix}cococo "foo""#))
        .expect_value_eq("foo")
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn redirect_combine(prefix: &str) -> Result {
    Playground::setup("redirect_combine", |dirs, _| {
        let code = format!("{prefix}echo_env_mixed out-err FOO BAR o+e>| str join ''");

        let actual: String = test()
            .env("FOO", "Foo")
            .env("BAR", "Bar")
            .cwd(dirs.test())
            .run(code)?;

        assert_eq!(actual, "Foo\nBar\n");
        Ok(())
    })
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_ps1_files(prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run_a_windows_ps_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.ps1",
            "
                Write-Host Hello World
            ",
        )]);

        let actual: String = test()
            .inherit_path()
            .inherit_env_if_set("SystemRoot")
            .cwd(dirs.test())
            .run(format!("{prefix}foo.ps1"))?;
        assert_contains("Hello World", actual);
        Ok(())
    })
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_ps1_files_with_space_in_path(prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("run_a_windows_ps_file", |dirs, sandbox| {
        sandbox
            .within("path with space")
            .with_files(&[FileWithContent(
                "foo.ps1",
                "
                    Write-Host Hello World
                ",
            )]);

        let actual: String = test()
            .inherit_path()
            .inherit_env_if_set("SystemRoot")
            .cwd(dirs.test().join("path with space"))
            .run(format!("{prefix}foo.ps1"))?;
        assert_contains("Hello World", actual);
        Ok(())
    })
}

#[rstest]
#[case::caret("^")]
#[case::run_external("run-external ")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn can_run_external_without_path_env(#[case] prefix: &str) -> Result {
    Playground::setup("can run external without path env", |dirs, _| {
        let bin = TESTBIN_COCOCO.path().to_string_lossy().into_owned();
        let code = format!(
            "
                hide-env -i PATH
                hide-env -i Path
                let bin = $in
                {prefix}$bin
            "
        );

        test()
            .cwd(dirs.test())
            .run_with_data(code, bin)
            .expect_value_eq("cococo")
    })
}

#[rstest]
#[case::caret("^")]
#[case::run_external("run-external ")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO, TESTBIN_MEOW)]
fn expand_command_if_list(#[case] prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("expand command if list", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("foo.txt", "Hello World")]);
        let actual: String = test()
            .cwd(dirs.test())
            .run(format!("let cmd = ['meow']; {prefix}$cmd foo.txt"))?;

        assert_contains("Hello World", actual);
        Ok(())
    })
}

#[rstest]
#[case::caret("^")]
#[case::run_external("run-external ")]
#[nu_test_support::test]
fn error_when_command_list_empty(#[case] prefix: &str) -> Result {
    Playground::setup("error when command is list with no items", |dirs, _| {
        let err = test()
            .cwd(dirs.test())
            .run(format!("let cmd = []; {prefix}$cmd"))
            .expect_shell_error()?;

        assert_contains("Missing parameter", err.to_string());
        Ok(())
    })
}
