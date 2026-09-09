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

#[test]
fn ordinary_list_stream_does_not_set_last_exit_code() -> Result {
    let code = "
        if ('LAST_EXIT_CODE' in ($env | columns)) { hide-env LAST_EXIT_CODE }
        let _ = (1..5 | each {|i| $i})
        1..5 | each {|i| $i}
        'LAST_EXIT_CODE' in ($env | columns)
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
fn complete_stream_rejects_internal_commands() -> Result {
    let err = test()
        .run("[1 2 3] | complete stream")
        .expect_shell_error()?;
    assert_matches!(err, ShellError::Generic(_));
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn complete_stream_tags_stdout_and_stderr_lines() -> Result {
    let code = "
        $env.FOO = 'hello'
        $env.BAR = 'world'
        let logs = (echo_env_mixed out-err FOO BAR | complete stream --lines)
        {
            stdout: ($logs | where stream == stdout | get chunk | str join)
            stderr: ($logs | where stream == stderr | get chunk | str join)
        }
    ";

    test().run(code).expect_value_eq(test_record! {
        "stdout" => "hello",
        "stderr" => "world",
    })
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn complete_stream_stdout_only() -> Result {
    test()
        .run("cococo test | complete stream --lines")
        .expect_value_eq(test_table![
            ["stream", "chunk"];
            ["stdout", "test"],
        ])
}

#[test]
#[deps(TESTBIN_FAIL)]
fn complete_stream_nonzero_exit_does_not_fail() -> Result {
    let code = "
        fail 42 | complete stream --lines
        $env.LAST_EXIT_CODE
    ";

    test().run(code).expect_value_eq(42)
}

#[test]
#[deps(TESTBIN_FAIL)]
fn complete_stream_sets_last_exit_code_after_collect() -> Result {
    let code = "
        let logs = (fail 7 | complete stream --lines)
        $env.LAST_EXIT_CODE
    ";

    test().run(code).expect_value_eq(7)
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_FAIL)]
fn complete_stream_does_not_raise_pipefail() -> Result {
    let code = "
        fail 3 | complete stream --lines
        $env.LAST_EXIT_CODE
    ";

    test().run(code).expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn complete_stream_first_does_not_hang() -> Result {
    let code = "
        $env.FOO = 'hello'
        $env.BAR = 'world'
        echo_env_mixed out-err FOO BAR | complete stream --lines | first | get chunk | is-empty
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
#[deps(TESTBIN_IECHO)]
fn complete_stream_first_on_infinite_stdout_does_not_hang() -> Result {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome: Result<String> =
            test().run("iecho y | complete stream --lines | first | get chunk");
        let _ = tx.send(outcome);
    });
    let outcome = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("complete stream | first hung on infinite stdout");
    assert_eq!(outcome?, "y");
    Ok(())
}

#[test]
#[deps(TESTBIN_IECHO)]
fn complete_stream_take_on_infinite_stdout_does_not_hang() -> Result {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome: Result<Vec<String>> =
            test().run("iecho y | complete stream --lines | take 3 | get chunk");
        let _ = tx.send(outcome);
    });
    let outcome = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("complete stream | take hung on infinite stdout");
    assert_eq!(outcome?, ["y", "y", "y"]);
    Ok(())
}

#[test]
#[exp(PIPE_FAIL)]
#[deps(TESTBIN_FAIL)]
fn complete_stream_wrapper_def_does_not_raise_pipefail() -> Result {
    let code = "
        def cs [] { complete stream --lines }
        fail 3 | cs
        $env.LAST_EXIT_CODE
    ";

    test().run(code).expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_FAIL)]
fn complete_stream_wrapper_def_sets_last_exit_code() -> Result {
    let code = "
        def cs [] { complete stream --lines }
        let logs = (fail 7 | cs)
        $env.LAST_EXIT_CODE
    ";

    test().run(code).expect_value_eq(7)
}

#[test]
#[deps(TESTBIN_FAIL)]
fn complete_stream_first_sets_last_exit_code() -> Result {
    let code = "
        fail 9 | complete stream --lines | first
        $env.LAST_EXIT_CODE
    ";

    test().run(code).expect_value_eq(9)
}

#[test]
#[deps(TESTBIN_FAIL)]
fn complete_stream_get_chunk_sets_last_exit_code() -> Result {
    let code = "
        fail 8 | complete stream --lines | get chunk
        $env.LAST_EXIT_CODE
    ";

    test().run(code).expect_value_eq(8)
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn complete_stream_wrapper_captures_stderr() -> Result {
    let code = "
        def cs [] { complete stream --lines }
        $env.FOO = 'hello'
        $env.BAR = 'world'
        let logs = (echo_env_mixed out-err FOO BAR | cs)
        {
            stdout: ($logs | where stream == stdout | get chunk | str join)
            stderr: ($logs | where stream == stderr | get chunk | str join)
        }
    ";

    test().run(code).expect_value_eq(test_record! {
        "stdout" => "hello",
        "stderr" => "world",
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn complete_stream_try_pipe_first_keeps_chunks() -> Result {
    let code = "
        $env.FOO = 'hello'
        $env.BAR = 'world'
        try { echo_env_mixed out-err FOO BAR | complete stream --lines } | first | get chunk | is-empty
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
#[deps(TESTBIN_IECHO)]
fn complete_stream_try_with_inner_first_does_not_hang() -> Result {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome: Result<String> =
            test().run("try { iecho y | complete stream --lines | first | get chunk }");
        let _ = tx.send(outcome);
    });
    let outcome = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("try { complete stream | first } hung on infinite stdout");
    assert_eq!(outcome?, "y");
    Ok(())
}

#[test]
#[deps(TESTBIN_REPEATER)]
fn complete_stream_lines_emits_oversize_fragment() -> Result {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome: Result<i64> = test()
            .run("repeater a 20000 | complete stream --lines | first | get chunk | str length");
        let _ = tx.send(outcome);
    });
    let outcome = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("complete stream --lines hung on newline-free input");
    assert_eq!(outcome?, 8192);
    Ok(())
}

#[test]
#[deps(NU)]
fn complete_stream_emits_partial_line_at_eof() -> Result {
    let code = r#"
        nu --no-config-file --commands "print -n leftover" | complete stream --lines
    "#;

    test().run(code).expect_value_eq(test_table![
        ["stream", "chunk"];
        ["stdout", "leftover"],
    ])
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn complete_stream_without_lines_contains_stdout() -> Result {
    test()
        .run("cococo hello | complete stream | get chunk | str join | str trim")
        .expect_value_eq("hello")
}
