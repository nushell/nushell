use nu_experimental::PIPE_FAIL;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
#[deps(TESTBIN_COCOCO)]
fn basic_stdout() -> Result {
    let mut tester = test();
    let without_complete: String = tester.run("cococo test")?;
    let with_complete: CompleteResult = tester.run("cococo test | complete")?;
    assert_eq!(without_complete.trim(), with_complete.stdout.trim());
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn basic_exit_code() -> Result {
    let result: CompleteResult = test().run("cococo test | complete")?;
    assert_eq!(result.exit_code, 0);
    Ok(())
}

#[test]
fn error() -> Result {
    let err = test().run("not-found | complete").expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::ExternalCommand { label, .. } if label == "Command `not-found` not found"
    );
    Ok(())
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_too_much_stderr_not_hang_nushell() -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stderr message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(&[FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual: CompleteResult = test()
            .inherit_path()
            .cwd(dirs.test())
            .run("sh -c 'cat a_large_file.txt 1>&2' | complete")?;

        assert_eq!(actual.stdout, "");
        assert_eq!(actual.stderr, large_file_body);
        assert_eq!(actual.exit_code, 0);
        Ok(())
    })
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_too_much_stdout_not_hang_nushell() -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup("external with many stdout message", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(&[FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual: CompleteResult = test()
            .inherit_path()
            .cwd(dirs.test())
            .run("sh -c 'cat a_large_file.txt' | complete")?;

        assert_eq!(actual.stdout, large_file_body);
        assert_eq!(actual.stderr, "");
        assert_eq!(actual.exit_code, 0);
        Ok(())
    })
}

#[test]
#[cfg(not(windows))]
fn capture_error_with_both_stdout_stderr_messages_not_hang_nushell() -> Result {
    use nu_test_support::fs::Stub::FileWithContent;
    use nu_test_support::playground::Playground;
    Playground::setup(
        "external with many stdout and stderr messages",
        |dirs, sandbox| {
            let script_body = "
        x=$(printf '=%.0s' $(seq 40960))
        echo $x
        echo $x 1>&2
        ";
            let expect_body = "=".repeat(40960);

            sandbox.with_files(&[FileWithContent("test.sh", script_body)]);

            let actual: CompleteResult = test()
                .inherit_path()
                .cwd(dirs.test())
                .run("sh test.sh | complete")?;

            assert_eq!(actual.stdout.trim(), expect_body);
            assert_eq!(actual.stderr.trim(), expect_body);
            assert_eq!(actual.exit_code, 0);
            Ok(())
        },
    )
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn combined_pipe_redirection() -> Result {
    let code = "
        $env.FOO = 'hello'; 
        $env.BAR = 'world'; 
        echo_env_mixed out-err FOO BAR o+e>| complete | get stdout
    ";

    test().run(code).expect_value_eq("hello\nworld\n")
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn err_pipe_redirection() -> Result {
    let actual: CompleteResult =
        test().run("$env.FOO = 'hello'; echo_env_stderr FOO e>| complete")?;
    assert_eq!(actual.stdout, "hello\n");
    assert_eq!(actual.stderr, "");
    assert_eq!(actual.exit_code, 0);
    Ok(())
}

#[rstest]
#[case::complete_parenthesized(r#"let result = (nu -n -c "exit 1" | complete)"#)]
#[case::complete(r#"let result = nu -n -c "exit 1" | complete"#)]
#[case::into_let(r#"nu -n -c "exit 1" | complete | let result"#)]
#[nu_test_support::test]
#[exp(PIPE_FAIL)]
#[deps(NU)]
fn pipefail_let(#[case] assignment: &str) -> Result {
    let mut tester = test();
    let _: Value = tester.run(assignment)?;
    let outcome: CompleteResult = tester.run("$result")?;
    assert_eq!(outcome.stdout, "");
    assert_eq!(outcome.stderr, "");
    assert_eq!(outcome.exit_code, 1);
    Ok(())
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(NU)]
fn pipefail_parenthesized_pipeline_let_keeps_scope() -> Result {
    let code = r#"
        (nu --no-config-file --commands "exit 1" | complete | let result);
        $result
    "#;

    let err = test().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::VariableNotFound { .. }));
    Ok(())
}
