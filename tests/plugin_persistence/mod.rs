//! The tests in this file check the soundness of plugin persistence. When a plugin is needed by Nu,
//! it is spawned only if it was not already running. Plugins that are spawned are kept running and
//! are referenced in the engine state. Plugins can be stopped by the user if desired, but not
//! removed.

// tests that assert stopping and starting or verify PIDs should run in serial to not clash with
// other test threads

use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
#[deps(NU_PLUGIN_INC, NU_PLUGIN_CUSTOM_VALUES)]
fn plugin_list_shows_installed_plugins() -> Result {
    test()
        .run("plugin list | get name | str join ','")
        .expect_value_eq("custom_values,inc")
}

#[test]
#[deps(NU_PLUGIN_INC)]
fn plugin_list_shows_installed_plugin_version() -> Result {
    test()
        .run("plugin list | get version.0")
        .expect_value_eq(env!("CARGO_PKG_VERSION"))
}

#[test]
#[serial]
#[deps(NU_PLUGIN_INC)]
fn plugin_keeps_running_after_calling_it() -> Result {
    let mut tester = test();
    let () = tester.run("plugin stop inc")?;
    tester
        .run("plugin list | get 0.status")
        .expect_value_eq("loaded")?;
    let _: Value = tester.run("'2.0.0' | inc -m")?;
    tester
        .run("plugin list | get 0.status")
        .expect_value_eq("running")
}

#[test]
#[serial]
#[deps(NU_PLUGIN_INC)]
fn plugin_process_exits_after_stop() -> Result {
    let code = r#"
        "2.0.0" | inc -m | ignore
        sleep 500ms

        let pid = plugin list | get 0.pid
        if (ps | where pid == $pid | is-empty) {
            error make {
                msg: "plugin process not running initially"
            }
        }

        plugin stop inc
        let start = date now
        mut cond = true
        while $cond {
            sleep 100ms
            $cond = (
                (ps | where pid == $pid | is-not-empty) and
                ((date now) - $start) < 5sec
            )
        }

        (date now) - $start | into int
    "#;

    let nanos: i64 = test().run(code)?;
    assert!(
        nanos < 5_000_000_000,
        "not stopped after more than 5 seconds: {nanos} ns"
    );

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_INC)]
fn plugin_stop_can_find_by_filename() -> Result {
    test()
        .run("plugin stop (plugin list | where name == inc).0.filename")
        .expect_value_eq(())
}

#[test]
#[serial]
#[deps(NU, NU_PLUGIN_INC)]
fn plugin_process_exits_when_nushell_exits() -> Result {
    // we have to run the nu binary to actually have a process exit
    let pid: u32 = test().run_with_data(
        r#"nu -n --plugins $in -c "'2.0.0' | inc -m; (plugin list).0.pid" | into int"#,
        NU_PLUGIN_INC.path(),
    )?;

    let mut tester = test();
    let () = tester.run("sleep 500ms")?;
    let _: Value = tester.run_with_data("let pid", pid)?;
    tester
        .run("ps | where pid == $pid | is-empty")
        .expect_value_eq(true)
}

#[rstest]
#[deps(NU_PLUGIN_INC)]
#[case::inc("'2.0.0' | inc -m | ignore")]
#[deps(NU_PLUGIN_EXAMPLE)]
#[case::example("example seq 1 10 | ignore")]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
#[case::custom_values("custom-value generate | ignore")]
#[nu_test_support::test]
fn plugin_commands_run_without_error(#[case] commands: &str) -> Result {
    test().run(commands).expect_value_eq(())
}

#[rstest]
#[deps(NU_PLUGIN_INC)]
#[case::inc_multiple_times(["'2.0.0' | inc -m","'2.1.0' | inc -m", "'2.2.0' | inc -m" ])]
#[deps(NU_PLUGIN_EXAMPLE)]
#[case::example_multiple_times(["example seq 1 10", "example seq 1 20"])]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
#[case::custom_values_multiple_times(["custom-value generate", "custom-value generate2"])]
#[nu_test_support::test]
fn plugin_commands_run_multiple_times_without_error(
    #[case] commands: impl IntoIterator<IntoIter = impl Iterator<Item = &'static str>>,
) -> Result {
    let mut tester = test();
    for commands in commands.into_iter() {
        let _: Value = tester.run(commands)?;
    }

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn multiple_plugin_commands_run_with_the_same_plugin_pid() -> Result {
    let mut tester = test();
    let () = tester.run("custom-value generate | ignore")?;
    let first: i64 = tester.run("plugin list | get 0.pid")?;
    let () = tester.run("custom-value generate2 | ignore")?;
    let second: i64 = tester.run("plugin list | get 0.pid")?;
    assert_eq!(first, second);
    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn plugin_pid_changes_after_stop_then_run_again() -> Result {
    let mut tester = test();
    let () = tester.run("custom-value generate | ignore")?;
    let first: i64 = tester.run("plugin list | get 0.pid")?;
    let () = tester.run("plugin stop custom_values")?;
    let () = tester.run("custom-value generate2 | ignore")?;
    let second: i64 = tester.run("plugin list | get 0.pid")?;
    assert_ne!(first, second);
    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn custom_values_can_still_be_passed_to_plugin_after_stop() -> Result {
    let code = "
        let cv = custom-value generate
        plugin stop custom_values
        $cv | custom-value update
    ";

    let value: Value = test().run(code)?;
    assert!(!value.is_nothing());
    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_CUSTOM_VALUES)]
fn custom_values_can_still_be_collapsed_after_stop() -> Result {
    let code = "
        let cv = custom-value generate
        plugin stop custom_values
        $cv | into value
    ";

    let value: String = test().run(code)?;
    assert!(!value.is_empty());
    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_INC)]
fn plugin_gc_can_be_configured_to_stop_plugins_immediately() -> Result {
    // I know the test is to stop "immediately", but if we actually check immediately it could
    // lead to a race condition. Using 100ms sleep just because with contention we don't really
    // know for sure how long this could take

    let code = r#"
        "2.3.0" | inc -M
        sleep 100ms
        (plugin list | where name == inc).0.status
    "#;

    let mut tester = test();
    let () = tester.run("$env.config.plugin_gc = { default: { stop_after: 0sec } }")?;
    tester.run(code).expect_value_eq("loaded")?;

    let mut tester = test();
    let () = tester.run("$env.config.plugin_gc = { plugins: { inc: { stop_after: 0sec } } }")?;
    tester.run(code).expect_value_eq("loaded")?;

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_INC)]
fn plugin_gc_can_be_configured_to_stop_plugins_after_delay() -> Result {
    let code = r#"
        "2.3.0" | inc -M

        let start = date now
        mut cond = true
        while $cond {
            sleep 100ms
            $cond = (
                (plugin list | where name == inc).0.status == running and
                ((date now) - $start) < 5sec
            )
        }

        ((date now) - $start) | into int
    "#;

    let mut tester = test();
    let () = tester.run("$env.config.plugin_gc = { default: { stop_after: 50ms } }")?;
    let nanos: i64 = tester.run(code)?;
    assert!(
        nanos < 5_000_000_000,
        "with config as default: more than 5 seconds: {nanos} ns"
    );

    let mut tester = test();
    let () = tester.run("$env.config.plugin_gc = { plugins: { inc: { stop_after: 50ms } } }")?;
    let nanos: i64 = tester.run(code)?;
    assert!(
        nanos < 5_000_000_000,
        "with inc-specific config: more than 5 seconds: {nanos} ns"
    );

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_INC)]
fn plugin_gc_can_be_configured_as_disabled() -> Result {
    let code = r#"
        $env.config.plugin_gc = { default: { enabled: false, stop_after: 0sec } }
        "2.3.0" | inc -M
        (plugin list | where name == inc).0.status == running
    "#;
    test().run(code).expect_value_eq(true)?;

    let code = r#"
        $env.config.plugin_gc = {
            default: { enabled: true, stop_after: 0sec }
            plugins: {
                inc: { enabled: false, stop_after: 0sec }
            }
        }
        "2.3.0" | inc -M
        (plugin list | where name == inc).0.status
    "#;
    test().run(code).expect_value_eq("running")?;

    Ok(())
}

#[test]
#[serial]
#[deps(NU_PLUGIN_EXAMPLE)]
fn plugin_gc_can_be_disabled_by_plugin() -> Result {
    let code = "
        example disable-gc
        $env.config.plugin_gc = { default: { stop_after: 0sec } }
        example one 1 foo | ignore # ensure we've run the plugin with the new config
        sleep 100ms
        (plugin list | where name == example).0.status
    ";

    test().run(code).expect_value_eq("running")
}

#[test]
#[serial]
#[deps(NU_PLUGIN_EXAMPLE)]
fn plugin_gc_does_not_stop_plugin_while_stream_output_is_active() -> Result {
    let code = "
        $env.config.plugin_gc = { default: { stop_after: 10ms } }
        # This would exceed the configured time
        example seq 1 500 | each { |n| sleep 1ms; $n } | length
    ";

    test().run(code).expect_value_eq(500)
}
