use std::{sync::mpsc, time::Duration};

use nu_test_support::nu_with_plugins;

fn ensure_stress_env_vars_unset() {
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("STRESS_") {
            panic!("Test is running in a dirty environment: {key:?} is set");
        }
    }
}

#[test]
fn test_stdio() {
    ensure_stress_env_vars_unset();
    let result = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_stress_internals"),
        "stress_internals"
    );
    assert!(result.status.success());
    assert!(result.out.contains("local_socket_path: None"));
}

#[test]
fn test_local_socket() {
    ensure_stress_env_vars_unset();
    let result = nu_with_plugins!(
        cwd: ".",
        envs: vec![
            ("STRESS_ADVERTISE_LOCAL_SOCKET", "1"),
        ],
        plugin: ("nu_plugin_stress_internals"),
        "stress_internals"
    );
    assert!(result.status.success());
    // Should be run once in stdio mode
    assert!(result.err.contains("--stdio"));
    // And then in local socket mode
    assert!(result.err.contains("--local-socket"));
    assert!(result.out.contains("local_socket_path: Some"));
}

#[test]
fn test_failing_local_socket_fallback() {
    ensure_stress_env_vars_unset();
    let result = nu_with_plugins!(
        cwd: ".",
        envs: vec![
            ("STRESS_ADVERTISE_LOCAL_SOCKET", "1"),
            ("STRESS_REFUSE_LOCAL_SOCKET", "1"),
        ],
        plugin: ("nu_plugin_stress_internals"),
        "stress_internals"
    );
    assert!(result.status.success());

    // Count the number of times we do stdio/local socket
    let mut count_stdio = 0;
    let mut count_local_socket = 0;

    for line in result.err.split('\n') {
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
    assert!(result.out.contains("local_socket_path: None"));
}

#[test]
fn test_exit_before_hello_stdio() {
    ensure_stress_env_vars_unset();
    // This can deadlock if not handled properly, so we try several times and timeout
    for _ in 0..5 {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = nu_with_plugins!(
                cwd: ".",
                envs: vec![
                    ("STRESS_EXIT_BEFORE_HELLO", "1"),
                ],
                plugin: ("nu_plugin_stress_internals"),
                "stress_internals"
            );
            let _ = tx.send(result);
        });
        let result = rx
            .recv_timeout(Duration::from_secs(15))
            .expect("timed out. probably a deadlock");
        assert!(!result.status.success());
    }
}

#[test]
fn test_exit_early_stdio() {
    ensure_stress_env_vars_unset();
    let result = nu_with_plugins!(
        cwd: ".",
        envs: vec![
            ("STRESS_EXIT_EARLY", "1"),
        ],
        plugin: ("nu_plugin_stress_internals"),
        "stress_internals"
    );
    assert!(!result.status.success());
    assert!(result.err.contains("--stdio"));
}

#[test]
fn test_exit_early_local_socket() {
    ensure_stress_env_vars_unset();
    let result = nu_with_plugins!(
        cwd: ".",
        envs: vec![
            ("STRESS_ADVERTISE_LOCAL_SOCKET", "1"),
            ("STRESS_EXIT_EARLY", "1"),
        ],
        plugin: ("nu_plugin_stress_internals"),
        "stress_internals"
    );
    assert!(!result.status.success());
    assert!(result.err.contains("--local-socket"));
}

#[test]
fn test_wrong_version() {
    ensure_stress_env_vars_unset();
    let result = nu_with_plugins!(
        cwd: ".",
        envs: vec![
            ("STRESS_WRONG_VERSION", "1"),
        ],
        plugin: ("nu_plugin_stress_internals"),
        "stress_internals"
    );
    assert!(!result.status.success());
    assert!(result.err.contains("version"));
    assert!(result.err.contains("0.0.0"));
}
