use nu_test_support::{nu, prelude::*};
use std::fs;

#[test]
fn ignore_still_causes_stream_to_be_consumed_fully() -> Result {
    Playground::setup("ignore_consumes_stream", |dirs, _| {
        let code = "
            [foo bar]
            | each {|val| $val | save --append output.txt; $val}
            | ignore
        ";

        let () = test().cwd(dirs.test()).run(code)?;
        let file_content = fs::read_to_string(dirs.test().join("output.txt")).unwrap();
        assert_eq!(file_content, "foobar");
        Ok(())
    })
}

#[test]
fn ignore_default_consumes_stdout_and_keeps_stderr() {
    let actual = nu!(
        r#"with-env { FOO: "message" } { nu --testbin echo_env_mixed out-err FOO FOO | ignore }"#
    );

    assert_eq!(actual.out, "");
    assert_eq!(actual.err, "message\n");
}

#[test]
fn ignore_stderr_consumes_stderr_and_allows_stdout() {
    let actual = nu!(
        r#"with-env { FOO: "message" } { nu --testbin echo_env_mixed out-err FOO FOO | ignore --stderr }"#
    );

    assert_eq!(actual.out, "message");
    assert_eq!(actual.err, "");
}

#[test]
fn ignore_stderr_allows_stdout_to_continue_in_pipeline() {
    let actual = nu!(
        r#"with-env { FOO: "message" } { nu --testbin echo_env_mixed out-err FOO FOO | ignore --stderr | str upcase }"#
    );

    assert_eq!(actual.out, "MESSAGE");
    assert_eq!(actual.err, "");
}

#[test]
fn ignore_with_stdout_and_stderr_consumes_both_streams() {
    let actual = nu!(
        r#"with-env { FOO: "message" } { nu --testbin echo_env_mixed out-err FOO FOO | ignore --stdout --stderr }"#
    );

    assert_eq!(actual.out, "");
    assert_eq!(actual.err, "");
}

#[test]
fn ignore_show_errors_allows_external_failures_and_sets_exit_code() {
    let actual =
        nu!("try { nu --testbin fail 42 | ignore --show-errors } catch { $env.LAST_EXIT_CODE }");

    assert_eq!(actual.out, "42");
}

#[test]
fn ignore_show_errors_sets_internal_failure_exit_code_to_one() {
    let actual = nu!(
        "try { error make {msg: 'boom'} | ignore --show-errors } catch { $env.LAST_EXIT_CODE }"
    );

    assert_eq!(actual.out, "1");
}

#[test]
fn ignore_stderr_with_show_errors_sets_internal_failure_exit_code_to_one() {
    let actual = nu!(
        "try { error make {msg: 'boom'} | ignore --stderr --show-errors } catch { $env.LAST_EXIT_CODE }"
    );

    assert_eq!(actual.out, "1");
}

#[test]
fn ignore_without_show_errors_does_not_set_last_exit_code() {
    let actual = nu!(
        "if ('LAST_EXIT_CODE' in ($env | columns)) { hide-env LAST_EXIT_CODE }; nu --testbin fail 42 | ignore; print done; $env | get --optional LAST_EXIT_CODE"
    );

    assert_eq!(actual.out, "done");
}

#[test]
fn ignore_stderr_suppresses_internal_errors() {
    let actual = nu!("ls this_path_does_not_exist_12345 | ignore --stderr");

    assert_eq!(actual.out, "");
    assert_eq!(actual.err, "");
}
