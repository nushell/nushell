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

#[test]
fn plugin_gc_can_be_configured_to_stop_plugins_immediately() {
    // I know the test is to stop "immediately", but if we actually check immediately it could
    // lead to a race condition. So there's a 1ms sleep just to be fair.
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            $env.config.plugin_gc = { default: { stop_after: 0sec } }
            "2.3.0" | inc -M
            sleep 1ms
            (plugin list | where name == inc).0.is_running
        "#
    );
    assert!(out.status.success());
    assert_eq!("false", out.out, "with config as default");

    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            $env.config.plugin_gc = {
                plugins: {
                    inc: { stop_after: 0sec }
                }
            }
            "2.3.0" | inc -M
            sleep 1ms
            (plugin list | where name == inc).0.is_running
        "#
    );
    assert!(out.status.success());
    assert_eq!("false", out.out, "with inc-specific config");
}

#[test]
fn plugin_gc_can_be_configured_to_stop_plugins_after_delay() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            $env.config.plugin_gc = { default: { stop_after: 50ms } }
            "2.3.0" | inc -M
            let start = (date now)
            mut cond = true
            while $cond {
                sleep 100ms
                $cond = (
                    (plugin list | where name == inc).0.is_running and
                    ((date now) - $start) < 5sec
                )
            }
            ((date now) - $start) | into int
        "#
    );
    assert!(out.status.success());
    let nanos = out.out.parse::<i64>().expect("not a number");
    assert!(
        nanos < 5_000_000_000,
        "with config as default: more than 5 seconds: {nanos} ns"
    );

    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            $env.config.plugin_gc = {
                plugins: {
                    inc: { stop_after: 50ms }
                }
            }
            "2.3.0" | inc -M
            let start = (date now)
            mut cond = true
            while $cond {
                sleep 100ms
                $cond = (
                    (plugin list | where name == inc).0.is_running and
                    ((date now) - $start) < 5sec
                )
            }
            ((date now) - $start) | into int
        "#
    );
    assert!(out.status.success());
    let nanos = out.out.parse::<i64>().expect("not a number");
    assert!(
        nanos < 5_000_000_000,
        "with inc-specific config: more than 5 seconds: {nanos} ns"
    );
}

#[test]
fn plugin_gc_can_be_configured_as_disabled() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            $env.config.plugin_gc = { default: { enabled: false, stop_after: 0sec } }
            "2.3.0" | inc -M
            (plugin list | where name == inc).0.is_running
        "#
    );
    assert!(out.status.success());
    assert_eq!("true", out.out, "with config as default");

    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_inc"),
        r#"
            $env.config.plugin_gc = {
                default: { enabled: true, stop_after: 0sec }
                plugins: {
                    inc: { enabled: false, stop_after: 0sec }
                }
            }
            "2.3.0" | inc -M
            (plugin list | where name == inc).0.is_running
        "#
    );
    assert!(out.status.success());
    assert_eq!("true", out.out, "with inc-specific config");
}

#[test]
fn plugin_gc_can_be_disabled_by_plugin() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_example"),
        r#"
            nu-example-disable-gc
            $env.config.plugin_gc = { default: { stop_after: 0sec } }
            nu-example-1 1 foo | ignore # ensure we've run the plugin with the new config
            sleep 10ms
            (plugin list | where name == example).0.is_running
        "#
    );
    assert!(out.status.success());
    assert_eq!("true", out.out);
}

#[test]
fn plugin_gc_does_not_stop_plugin_while_stream_output_is_active() {
    let out = nu_with_plugins!(
        cwd: ".",
        plugin: ("nu_plugin_stream_example"),
        r#"
            $env.config.plugin_gc = { default: { stop_after: 10ms } }
            # This would exceed the configured time
            stream_example seq 1 500 | each { |n| sleep 1ms; $n } | length | print
        "#
    );
    assert!(out.status.success());
    assert_eq!("500", out.out);
}
