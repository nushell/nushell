use std::fs;

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
fn explicit_glob(#[ignore] playground: Playground, prefix: &str) -> Result {
    playground.empty_file("D&D_volume_1.txt")?;
    playground.empty_file("D&D_volume_2.txt")?;
    playground.empty_file("foo.sh")?;

    let actual: String = test()
        .cwd(playground.path())
        .run(format!("{prefix}cococo ('*.txt' | into glob)"))?;

    assert_contains("D&D_volume_1.txt", &actual);
    assert_contains("D&D_volume_2.txt", actual);
    Ok(())
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn bare_word_expand_path_glob(#[ignore] playground: Playground, prefix: &str) -> Result {
    playground.empty_file("D&D_volume_1.txt")?;
    playground.empty_file("D&D_volume_2.txt")?;
    playground.empty_file("foo.sh")?;

    let actual: String = test()
        .cwd(playground.path())
        .run(format!("{prefix}cococo *.txt"))?;

    assert_contains("D&D_volume_1.txt", &actual);
    assert_contains("D&D_volume_2.txt", actual);
    Ok(())
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn backtick_expand_path_glob(#[ignore] playground: Playground, prefix: &str) -> Result {
    playground.empty_file("D&D_volume_1.txt")?;
    playground.empty_file("D&D_volume_2.txt")?;
    playground.empty_file("foo.sh")?;

    let actual: String = test()
        .cwd(playground.path())
        .run(format!("{prefix}cococo `*.txt`"))?;

    assert_contains("D&D_volume_1.txt", &actual);
    assert_contains("D&D_volume_2.txt", actual);
    Ok(())
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn single_quote_does_not_expand_path_glob(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    playground.empty_file("D&D_volume_1.txt")?;
    playground.empty_file("D&D_volume_2.txt")?;
    playground.empty_file("foo.sh")?;

    test()
        .cwd(playground.path())
        .run(format!("{prefix}cococo '*.txt'"))
        .expect_value_eq("*.txt")
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn double_quote_does_not_expand_path_glob(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    playground.empty_file("D&D_volume_1.txt")?;
    playground.empty_file("D&D_volume_2.txt")?;
    playground.empty_file("foo.sh")?;

    test()
        .cwd(playground.path())
        .run(format!(r#"{prefix}cococo "*.txt""#))
        .expect_value_eq("*.txt")
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_FAIL)]
fn failed_command_with_semicolon_will_not_execute_following_cmds(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    let code = format!("try {{ {prefix}fail; echo done }} catch {{ 'stopped' }}");

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("stopped")
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_args_with_quoted(#[ignore] playground: Playground, prefix: &str) -> Result {
    test()
        .cwd(playground.path())
        .run(format!(r#"{prefix}cococo "foo=bar 'hi'""#))
        .expect_value_eq("foo=bar 'hi'")
}

#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_with_option_like_embedded_quotes(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    test()
        .cwd(playground.path())
        .run(format!("{prefix}cococo -- --foo='bar' -foo='bar'"))
        .expect_value_eq("--foo=bar -foo=bar")
}

// FIXME: parser complains about invalid characters after single quote
#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_with_non_option_like_embedded_quotes(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    test()
        .cwd(playground.path())
        .run(format!("{prefix}cococo foo='bar' 'foo'=bar"))
        .expect_value_eq("foo=bar foo=bar")
}

// FIXME: parser bug prevents expressions from appearing within GlobPattern substrings
#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_with_string_interpolation(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    let code = format!(r#"{prefix}cococo foo=(2 + 2) $"foo=(2 + 2)" foo=$"(2 + 2)""#);

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("foo=4 foo=4 foo=4")
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_NONU)]
fn external_arg_with_variable_name(#[ignore] playground: Playground, prefix: &str) -> Result {
    let code = format!(
        r#"
            let dump_command = "PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'"
            {prefix}nonu $dump_command
        "#
    );

    test().cwd(playground.path()).run(code).expect_value_eq(
        "PGPASSWORD='db_secret' pg_dump -Fc -h 'db.host' -p '$db.port' -U postgres -d 'db_name' > '/tmp/dump_name'",
    )
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_command_escape_args(#[ignore] playground: Playground, prefix: &str) -> Result {
    test()
        .cwd(playground.path())
        .run(format!(r#"{prefix}cococo "\"abcd""#))
        .expect_value_eq(r#""abcd"#)
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
#[deps(NU, TESTBIN_COCOCO)]
fn external_command_expand_tilde(#[ignore] playground: Playground, prefix: &str) -> Result {
    // Make a copy of the testbin that can be found through tilde expansion.
    let testbin_path = playground.path().join("test_cococo");
    fs::copy(TESTBIN_COCOCO.path(), &testbin_path)?;

    // For this to work the process needs to have the `HOME` env set,
    // but we only get the path via the playground, so we cannot attribute this test function.
    test()
        .env("HOME", playground.path())
        .run(format!("nu -n -c '{prefix}~/test_cococo hello'"))
        .expect_value_eq("hello")
}

// FIXME: parser bug prevents expressions from appearing within GlobPattern substrings
#[apply(direct_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn external_arg_expand_tilde(#[ignore] playground: Playground, prefix: &str) -> Result {
    let home = dirs::home_dir().expect("failed to find home dir");
    test()
        .cwd(playground.path())
        .run(format!("{prefix}cococo ~/foo ~/(2 + 2)"))
        .expect_value_eq(format!(
            "{} {}",
            home.join("foo").display(),
            home.join("4").display()
        ))
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_NONU)]
fn external_command_not_expand_tilde_with_quotes(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    test()
        .cwd(playground.path())
        .run(format!(r#"{prefix}nonu "~""#))
        .expect_value_eq("~")
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_NONU)]
fn external_command_expand_tilde_with_back_quotes(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    let actual: String = test()
        .cwd(playground.path())
        .run(format!("{prefix}nonu `~`"))?;
    assert_contains_not("~", actual);
    Ok(())
}

#[apply(run_external_prefixes)]
#[nu_test_support::test]
#[deps(TESTBIN_INPUT_BYTES_LENGTH)]
fn external_command_receives_raw_binary_data(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    test()
        .cwd(playground.path())
        .run(format!("0x[deadbeef] | {prefix}input_bytes_length"))
        .expect_value_eq("4")
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_cmd_files(#[ignore] playground: Playground, prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    playground.file(
        "foo.cmd",
        "
            @echo off
            echo Hello World
        ",
    )?;

    let actual: String = test()
        .cwd(playground.path())
        .run(format!("{prefix}foo.cmd"))?;
    assert_contains("Hello World", actual);
    Ok(())
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_batch_files(#[ignore] playground: Playground, prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    playground.file(
        "foo.bat",
        "
            @echo off
            echo Hello World
        ",
    )?;

    let actual: String = test()
        .cwd(playground.path())
        .run(format!("{prefix}foo.bat"))?;
    assert_contains("Hello World", actual);
    Ok(())
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_batch_files_without_cmd_extension(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    playground.file(
        "foo.cmd",
        "
                @echo off
                echo Hello World
            ",
    )?;

    let actual: String = test().cwd(playground.path()).run(format!("{prefix}foo"))?;
    assert_contains("Hello World", actual);
    Ok(())
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_batch_files_without_bat_extension(
    #[ignore] playground: Playground,
    prefix: &str,
) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    playground.file(
        "foo.bat",
        "
                @echo off
                echo Hello World
            ",
    )?;

    let actual: String = test().cwd(playground.path()).run(format!("{prefix}foo"))?;
    assert_contains("Hello World", actual);
    Ok(())
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
fn redirect_combine(#[ignore] playground: Playground, prefix: &str) -> Result {
    let code = format!("{prefix}echo_env_mixed out-err FOO BAR o+e>| str join ''");

    let actual: String = test()
        .env("FOO", "Foo")
        .env("BAR", "Bar")
        .cwd(playground.path())
        .run(code)?;

    assert_eq!(actual, "Foo\nBar\n");
    Ok(())
}

#[cfg(windows)]
#[apply(run_external_prefixes)]
#[nu_test_support::test]
fn can_run_ps1_files(#[ignore] playground: Playground, prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    playground.file(
        "foo.ps1",
        "
            Write-Host Hello World
        ",
    )?;

    let actual: String = test()
        .inherit_path()
        .inherit_env_if_set("SystemRoot")
        .cwd(playground.path())
        .run(format!("{prefix}foo.ps1"))?;
    assert_contains("Hello World", actual);
    Ok(())
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
fn can_run_external_without_path_env(
    #[ignore] playground: Playground,
    #[case] prefix: &str,
) -> Result {
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
        .cwd(playground.path())
        .run_with_data(code, bin)
        .expect_value_eq("cococo")
}

#[rstest]
#[case::caret("^")]
#[case::run_external("run-external ")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO, TESTBIN_MEOW)]
fn expand_command_if_list(#[ignore] playground: Playground, #[case] prefix: &str) -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    playground.file("foo.txt", "Hello World")?;
    let actual: String = test()
        .cwd(playground.path())
        .run(format!("let cmd = ['meow']; {prefix}$cmd foo.txt"))?;

    assert_contains("Hello World", actual);
    Ok(())
}

#[rstest]
#[case::caret("^")]
#[case::run_external("run-external ")]
#[nu_test_support::test]
fn error_when_command_list_empty(#[ignore] playground: Playground, #[case] prefix: &str) -> Result {
    let err = test()
        .cwd(playground.path())
        .run(format!("let cmd = []; {prefix}$cmd"))
        .expect_shell_error()?;

    assert_contains("Missing parameter", err.to_string());
    Ok(())
}
