use nu_experimental::PIPE_FAIL;
use nu_test_support::prelude::*;
use rstest::rstest;
use std::collections::HashMap;

#[test]
fn basic_stdout() -> Result {
    let mut tester = test().add_nu_to_path();
    let without_complete: String = tester.run("nu --testbin cococo test")?;
    let with_complete: String = tester.run("(nu --testbin cococo test | complete).stdout")?;
    assert_eq!(without_complete.trim(), with_complete.trim());
    Ok(())
}

#[test]
fn basic_exit_code() -> Result {
    test()
        .add_nu_to_path()
        .run("(nu --testbin cococo test | complete).exit_code")
        .expect_value_eq(0)
}

#[test]
fn error() -> Result {
    let err = test().run("not-found | complete").expect_shell_error()?;
    match err {
        ShellError::ExternalCommand { label, .. } => {
            assert_eq!(label, "Command `not-found` not found");
            Ok(())
        }
        err => Err(err.into()),
    }
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

        let actual: String = test()
            .inherit_path()
            .cwd(dirs.test())
            .run("sh -c 'cat a_large_file.txt 1>&2' | complete | get stderr")?;

        assert_eq!(actual, large_file_body);
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

        let actual: String = test()
            .inherit_path()
            .cwd(dirs.test())
            .run("sh -c 'cat a_large_file.txt' | complete | get stdout")?;

        assert_eq!(actual, large_file_body);
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

            // check for stdout
            let actual: String = test()
                .inherit_path()
                .cwd(dirs.test())
                .run("sh test.sh | complete | get stdout | str trim")?;
            assert_eq!(actual, expect_body);
            // check for stderr
            let actual: String = test()
                .inherit_path()
                .cwd(dirs.test())
                .run("sh test.sh | complete | get stderr | str trim")?;
            assert_eq!(actual, expect_body);
            Ok(())
        },
    )
}

#[test]
fn combined_pipe_redirection() -> Result {
    let code = "
        $env.FOO = 'hello'; 
        $env.BAR = 'world'; 
        nu --testbin echo_env_mixed out-err FOO BAR o+e>| complete | get stdout
    ";

    test()
        .add_nu_to_path()
        .run(code)
        .expect_value_eq("hello\nworld\n")
}

#[test]
fn err_pipe_redirection() -> Result {
    test()
        .add_nu_to_path()
        .run("$env.FOO = 'hello'; nu --testbin echo_env_stderr FOO e>| complete | get stdout")
        .expect_value_eq("hello\n")
}

#[rstest]
#[case::complete_parenthesized(r#"let result = (nu -n -c "exit 1" | complete)"#)]
#[case::complete(r#"let result = nu -n -c "exit 1" | complete"#)]
#[case::into_let(r#"nu -n -c "exit 1" | complete | let result"#)]
#[nu_test_support::test]
#[exp(PIPE_FAIL)]
fn pipefail_let(#[case] assignment: &str) -> Result {
    let mut tester = test().add_nu_to_path();
    let _: Value = tester.run(assignment)?;
    let outcome: HashMap<String, Value> = tester.run("$result")?;
    outcome["stdout"].assert_eq("");
    outcome["stderr"].assert_eq("");
    outcome["exit_code"].assert_eq(1);
    Ok(())
}

#[test]
#[exp(PIPE_FAIL)]
fn pipefail_parenthesized_pipeline_let_keeps_scope() -> Result {
    let code = r#"
        (nu --no-config-file --commands "exit 1" | complete | let result);
        $result
    "#;

    let err = test().add_nu_to_path().run(code).expect_parse_error()?;
    assert!(matches!(err, ParseError::VariableNotFound { .. }));
    Ok(())
}
