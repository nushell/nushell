// all tests in here are marked as serial to give your processor some room to breath while
// executing these tests

use nu_test_support::prelude::*;

#[derive(FromValue)]
struct CompleteResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
}

const CODE_STRESS_INTERNALS_COMPLETE: &str = r#"
    let plugin_path = (which nu_plugin_stress_internals).path.0
    nu --no-config-file --plugins $plugin_path --commands "stress_internals" | complete
"#;

#[test]
#[serial]
#[deps(NU_PLUGIN_STRESS_INTERNALS)]
fn test_stdio() -> Result {
    let result: CompleteResult = test().run(CODE_STRESS_INTERNALS_COMPLETE)?;
    assert_contains("local_socket_path: None", &result.stdout);
    Ok(())
}

#[test]
#[serial]
#[deps(NU, NU_PLUGIN_STRESS_INTERNALS)]
fn test_local_socket() -> Result {
    let result: CompleteResult = test()
        .env("STRESS_ADVERTISE_LOCAL_SOCKET", "1")
        .run(CODE_STRESS_INTERNALS_COMPLETE)?;

    assert_eq!(result.exit_code, 0);

    // Should be run once in stdio mode
    assert_contains("--stdio", &result.stderr);
    // And then in local socket mode
    assert_contains("--local-socket", &result.stderr);
    assert_contains("local_socket_path: Some", &result.stdout);

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_STRESS_INTERNALS)]
fn test_failing_local_socket_fallback() -> Result {
    let result: CompleteResult = test()
        .env("STRESS_ADVERTISE_LOCAL_SOCKET", "1")
        .env("STRESS_REFUSE_LOCAL_SOCKET", "1")
        .run(CODE_STRESS_INTERNALS_COMPLETE)?;

    assert_eq!(result.exit_code, 0);

    // Count the number of times we do stdio/local socket
    let mut count_stdio = 0;
    let mut count_local_socket = 0;

    for line in result.stderr.lines() {
        if line.contains("--stdio") {
            count_stdio += 1;
        }
        if line.contains("--local-socket") {
            count_local_socket += 1;
        }
    }

    // Should be run once in local socket mode
    assert_eq!(1, count_local_socket, "count of --local-socket");
    // Should be run twice in stdio mode, due to the fallback
    assert_eq!(2, count_stdio, "count of --stdio");

    // In the end it should not be running in local socket mode, but should succeed
    assert_contains("local_socket_path: None", &result.stdout);

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_STRESS_INTERNALS)]
fn test_exit_before_hello_stdio() -> Result {
    let result: CompleteResult = test()
        .env("STRESS_EXIT_EARLY", "1")
        .run(CODE_STRESS_INTERNALS_COMPLETE)?;
    assert_ne!(result.exit_code, 0);
    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_STRESS_INTERNALS)]
fn test_exit_early_stdio() -> Result {
    let result: CompleteResult = test()
        .env("STRESS_EXIT_EARLY", "1")
        .run(CODE_STRESS_INTERNALS_COMPLETE)?;

    assert_ne!(result.exit_code, 0);
    assert_contains("--stdio", &result.stderr);

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_STRESS_INTERNALS)]
fn test_exit_early_local_socket() -> Result {
    let result: CompleteResult = test()
        .env("STRESS_ADVERTISE_LOCAL_SOCKET", "1")
        .env("STRESS_EXIT_EARLY", "1")
        .run(CODE_STRESS_INTERNALS_COMPLETE)?;

    assert_ne!(result.exit_code, 0);
    assert_contains("--local-socket", &result.stderr);

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_STRESS_INTERNALS)]
fn test_wrong_version() -> Result {
    let result: CompleteResult = test()
        .env("STRESS_WRONG_VERSION", "1")
        .run(CODE_STRESS_INTERNALS_COMPLETE)?;

    assert_ne!(result.exit_code, 0);
    assert_contains("version", &result.stderr);
    assert_contains("0.0.0", &result.stderr);

    Ok(())
}
