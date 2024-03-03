//! The tests in this file check the soundness of plugin persistence. When a plugin is needed by Nu,
//! it is spawned only if it was not already running. Plugins that are spawned are kept running and
//! are referenced in the engine state. Plugins can be stopped by the user if desired, but not
//! removed.

use nu_test_support::{nu, nu_with_plugins};

#[test]
fn plugin_list_shows_installed_plugins() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugins: [("nu_plugin_inc"), ("nu_plugin_custom_values")],
        r#"(plugin list).name | str join ','"#
    );
    assert_eq!("inc,custom_values", out.out);
    assert!(out.status.success());
}

#[test]
fn plugin_keeps_running_after_calling_it() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            plugin stop inc
            (plugin list).0.is_running | print
            print ";"
            "2.0.0" | inc -m | ignore
            (plugin list).0.is_running | print
        "#
    );
    assert_eq!(
        "false;true", out.out,
        "plugin list didn't show is_running = true"
    );
    assert!(out.status.success());
}

#[test]
fn plugin_process_exits_after_stop() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            "2.0.0" | inc -m | ignore
            let pid = (plugin list).0.pid
            ps | where pid == $pid | length | print
            print ";"
            plugin stop inc
            sleep 10ms
            ps | where pid == $pid | length | print
        "#
    );
    assert_eq!("1;0", out.out, "plugin process did not stop running");
    assert!(out.status.success());
}

#[test]
fn plugin_process_exits_when_nushell_exits() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            "2.0.0" | inc -m | ignore
            (plugin list).0.pid | print
        "#
    );
    assert!(!out.out.is_empty());
    assert!(out.status.success());

    let pid = out.out.parse::<u32>().expect("failed to parse pid");

    // use nu to check if process exists
    assert_eq!(
        "0",
        nu!(format!("ps | where pid == {pid} | length")).out,
        "plugin process {pid} is still running"
    );
}

#[test]
fn plugin_commands_run_without_error() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugins: [
            ("nu_plugin_inc"),
            ("nu_plugin_stream_example"),
            ("nu_plugin_custom_values"),
        ],
        r#"
            "2.0.0" | inc -m | ignore
            stream_example seq 1 10 | ignore
            custom-value generate | ignore
        "#
    );
    assert!(out.err.is_empty());
    assert!(out.status.success());
}

#[test]
fn plugin_commands_run_multiple_times_without_error() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugins: [
            ("nu_plugin_inc"),
            ("nu_plugin_stream_example"),
            ("nu_plugin_custom_values"),
        ],
        r#"
            ["2.0.0" "2.1.0" "2.2.0"] | each { inc -m } | print
            stream_example seq 1 10 | ignore
            custom-value generate | ignore
            stream_example seq 1 20 | ignore
            custom-value generate2 | ignore
        "#
    );
    assert!(out.err.is_empty());
    assert!(out.status.success());
}

#[test]
fn multiple_plugin_commands_run_with_the_same_plugin_pid() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_custom_values"),
        r#"
            custom-value generate | ignore
            (plugin list).0.pid | print
            print ";"
            custom-value generate2 | ignore
            (plugin list).0.pid | print
        "#
    );
    assert!(out.status.success());

    let pids: Vec<&str> = out.out.split(';').collect();
    assert_eq!(2, pids.len());
    assert_eq!(pids[0], pids[1]);
}

#[test]
fn plugin_pid_changes_after_stop_then_run_again() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_custom_values"),
        r#"
            custom-value generate | ignore
            (plugin list).0.pid | print
            print ";"
            plugin stop custom_values
            custom-value generate2 | ignore
            (plugin list).0.pid | print
        "#
    );
    assert!(out.status.success());

    let pids: Vec<&str> = out.out.split(';').collect();
    assert_eq!(2, pids.len());
    assert_ne!(pids[0], pids[1]);
}

#[test]
fn custom_values_can_still_be_passed_to_plugin_after_stop() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_custom_values"),
        r#"
            let cv = custom-value generate
            plugin stop custom_values
            $cv | custom-value update
        "#
    );
    assert!(!out.out.is_empty());
    assert!(out.err.is_empty());
    assert!(out.status.success());
}

#[test]
fn custom_values_can_still_be_collapsed_after_stop() {
    // print causes a collapse (ToBaseValue) call.
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_custom_values"),
        r#"
            let cv = custom-value generate
            plugin stop custom_values
            $cv | print
        "#
    );
    assert!(!out.out.is_empty());
    assert!(out.err.is_empty());
    assert!(out.status.success());
}
