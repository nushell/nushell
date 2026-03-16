use nu_experimental::PIPE_FAIL;
use nu_test_support::{prelude::*, tester::NuTester};
use std::collections::HashMap;

fn built_nu_path() -> String {
    nu_test_support::fs::executable_path().display().to_string()
}

fn tester_with_path() -> NuTester {
    test().inherit_path().add_nu_to_path()
}

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
            let script_body = r#"
        x=$(printf '=%.0s' $(seq 40960))
        echo $x
        echo $x 1>&2
        "#;
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
    tester_with_path()
        .run("$env.FOO = 'hello'; nu --testbin echo_env_stderr FOO e>| complete | get stdout")
        .expect_value_eq("hello\n")
}

#[test]
#[exp(PIPE_FAIL)]
fn pipefail_let_with_parenthesized_complete_assignment() -> Result {
    let command = r#"
        let result = (nu --no-config-file --commands "exit 1" | complete); 
        $result
    "#;

    let outcome: HashMap<String, Value> = test().add_nu_to_path().run(command)?;
    outcome["stdout"].assert_value_eq("");
    outcome["stderr"].assert_value_eq("");
    outcome["exit_code"].assert_value_eq(1);
    Ok(())
}

#[test]
fn pipefail_let_with_complete_assignment() {
    let nu_path = built_nu_path();
    let command = format!(
        "let result = ^{nu_path} --no-config-file --commands \"exit 1\" | complete; print $result"
    );
    let actual = nu!(experimental: vec!["pipefail".to_string()], command);

    assert!(actual.out.contains("exit_code"));
    assert!(!actual.err.contains("non_zero_exit_code"));
}

#[test]
fn pipefail_pipeline_complete_into_let() {
    let nu_path = built_nu_path();
    let command = format!(
        "^{nu_path} --no-config-file --commands \"exit 1\" | complete | let result; print $result"
    );
    let actual = nu!(experimental: vec!["pipefail".to_string()], command);

    assert!(actual.out.contains("exit_code"));
    assert!(!actual.err.contains("non_zero_exit_code"));
}

#[test]
fn pipefail_parenthesized_pipeline_let_keeps_scope() {
    let nu_path = built_nu_path();
    let command = format!(
        "(^{nu_path} --no-config-file --commands \"exit 1\" | complete | let result); print $result"
    );
    let actual = nu!(experimental: vec!["pipefail".to_string()], command);

    assert!(actual.err.contains("nu::parser::variable_not_found"));
    assert!(!actual.err.contains("nu::shell::non_zero_exit_code"));
}
