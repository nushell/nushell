use nu_test_support::prelude::*;
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
        let file_content = fs::read_to_string(dirs.test().join("output.txt"))?;
        assert_eq!(file_content, "foobar");
        Ok(())
    })
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn ignore_default_consumes_stdout_and_keeps_stderr() -> Result {
    let child_code = r#"$env.FOO = "message"; echo_env_mixed out-err FOO FOO | ignore"#;
    let actual: CompleteResult = test().run_with_data(
        "let child_code; nu -n -c $child_code | complete",
        child_code,
    )?;

    assert_eq!(actual.stdout, "");
    assert_eq!(actual.stderr, "message\n");
    assert_eq!(actual.exit_code, 0);
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn ignore_stderr_consumes_stderr_and_allows_stdout() -> Result {
    let child_code = r#"$env.FOO = "message"; echo_env_mixed out-err FOO FOO | ignore --stderr"#;
    let actual: CompleteResult = test().run_with_data(
        "let child_code; nu -n -c $child_code | complete",
        child_code,
    )?;

    assert_eq!(actual.stdout, "message\n");
    assert_eq!(actual.stderr, "");
    assert_eq!(actual.exit_code, 0);
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn ignore_stderr_allows_stdout_to_continue_in_pipeline() -> Result {
    let child_code = r#"
        $env.FOO = "message"
        echo_env_mixed out-err FOO FOO | ignore --stderr | str uppercase
    "#;
    let actual: CompleteResult = test().run_with_data(
        "let child_code; nu -n -c $child_code | complete",
        child_code,
    )?;

    assert_eq!(actual.stdout, "MESSAGE\n");
    assert_eq!(actual.stderr, "");
    assert_eq!(actual.exit_code, 0);
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn ignore_with_stdout_and_stderr_consumes_both_streams() -> Result {
    let child_code =
        r#"$env.FOO = "message"; echo_env_mixed out-err FOO FOO | ignore --stdout --stderr"#;
    let actual: CompleteResult = test().run_with_data(
        "let child_code; nu -n -c $child_code | complete",
        child_code,
    )?;

    assert_eq!(actual.stdout, "");
    assert_eq!(actual.stderr, "");
    assert_eq!(actual.exit_code, 0);
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn ignore_show_errors_allows_external_failures_and_sets_exit_code() -> Result {
    test()
        .run("try { fail 42 | ignore --show-errors } catch { $env.LAST_EXIT_CODE }")
        .expect_value_eq(42)
}

#[test]
fn ignore_show_errors_sets_internal_failure_exit_code_to_one() -> Result {
    test()
        .run(
            "try { error make {msg: 'boom'} | ignore --show-errors } catch { $env.LAST_EXIT_CODE }",
        )
        .expect_value_eq(1)
}

#[test]
fn ignore_stderr_with_show_errors_sets_internal_failure_exit_code_to_one() -> Result {
    let code = "try { error make {msg: 'boom'} | ignore --stderr --show-errors } catch { $env.LAST_EXIT_CODE }";

    test().run(code).expect_value_eq(1)
}

#[test]
#[deps(TESTBIN_FAIL)]
fn ignore_without_show_errors_does_not_set_last_exit_code() -> Result {
    let code = "
        if ('LAST_EXIT_CODE' in ($env | columns)) { hide-env LAST_EXIT_CODE }
        fail 42 | ignore
        'LAST_EXIT_CODE' in ($env | columns)
    ";

    test().run(code).expect_value_eq(false)
}

#[test]
fn ignore_stderr_suppresses_internal_errors() -> Result {
    test()
        .run("ls this_path_does_not_exist_12345 | ignore --stderr")
        .expect_value_eq(())
}
