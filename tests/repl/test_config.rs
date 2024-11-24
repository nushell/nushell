use crate::repl::tests::{fail_test, run_test, run_test_std, TestResult};
use nu_test_support::nu;

#[test]
fn mutate_nu_config() -> TestResult {
    run_test_std(
        r#"$env.config.footer_mode = 30; $env.config.footer_mode"#,
        "30",
    )
}

#[test]
fn mutate_nu_config_nested_ls() -> TestResult {
    run_test_std(
        r#"$env.config.ls.clickable_links = false; $env.config.ls.clickable_links"#,
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_table() -> TestResult {
    run_test_std(
        r#"
            $env.config.table.trim.methodology = 'wrapping'
            $env.config.table.trim.wrapping_try_keep_words = false
            $env.config.table.trim.wrapping_try_keep_words
        "#,
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_menu() -> TestResult {
    run_test_std(
        r#"
            $env.config.menus = [
                {
                  name: menu
                  only_buffer_difference: true
                  marker: "M "
                  type: {}
                  style: {}
                }
            ];
            $env.config.menus.0.type.columns = 3;
            $env.config.menus.0.type.columns
        "#,
        "3",
    )
}

#[test]
fn mutate_nu_config_nested_keybindings() -> TestResult {
    run_test_std(
        r#"
            $env.config.keybindings = [
                {
                  name: completion_previous
                  modifier: shift
                  keycode: backtab
                  mode: [ vi_normal, vi_insert ]
                  event: { send: menuprevious }
                }
            ];
            $env.config.keybindings.0.keycode = 'char_x';
            $env.config.keybindings.0.keycode
        "#,
        "char_x",
    )
}

#[test]
fn mutate_nu_config_nested_color_nested() -> TestResult {
    run_test_std(
        r#"$env.config.color_config.shape_flag = 'cyan'; $env.config.color_config.shape_flag"#,
        "cyan",
    )
}

#[test]
fn mutate_nu_config_nested_completion() -> TestResult {
    run_test_std(
        r#"$env.config.completions.external.enable = false; $env.config.completions.external.enable"#,
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_history() -> TestResult {
    run_test_std(
        r#"$env.config.history.max_size = 100; $env.config.history.max_size"#,
        "100",
    )
}

#[test]
fn mutate_nu_config_nested_filesize() -> TestResult {
    run_test_std(
        r#"$env.config.filesize.format = 'kb'; $env.config.filesize.format"#,
        "kb",
    )
}

#[test]
fn mutate_nu_config_plugin() -> TestResult {
    run_test_std(
        r#"
            $env.config.plugins = {
                config: {
                  key1: value
                  key2: other
                }
            };
            $env.config.plugins.config.key1 = 'updated'
            $env.config.plugins.config.key1
        "#,
        "updated",
    )
}

#[test]
fn reject_nu_config_plugin_non_record() -> TestResult {
    fail_test(r#"$env.config.plugins = 5"#, "Type mismatch")
}

#[test]
fn mutate_nu_config_plugin_gc_default_enabled() -> TestResult {
    run_test(
        r#"
            $env.config.plugin_gc.default.enabled = false
            $env.config.plugin_gc.default.enabled
        "#,
        "false",
    )
}

#[test]
fn mutate_nu_config_plugin_gc_default_stop_after() -> TestResult {
    run_test(
        r#"
            $env.config.plugin_gc.default.stop_after = 20sec
            $env.config.plugin_gc.default.stop_after
        "#,
        "20sec",
    )
}

#[test]
fn mutate_nu_config_plugin_gc_default_stop_after_negative() -> TestResult {
    fail_test(
        r#"
            $env.config.plugin_gc.default.stop_after = -1sec
            $env.config.plugin_gc.default.stop_after
        "#,
        "expected a non-negative duration",
    )
}

#[test]
fn mutate_nu_config_plugin_gc_plugins() -> TestResult {
    run_test(
        r#"
            $env.config.plugin_gc.plugins.inc = {
                enabled: true
                stop_after: 0sec
            }
            $env.config.plugin_gc.plugins.inc.stop_after
        "#,
        "0sec",
    )
}

#[test]
fn env_is_not_loaded_by_commandstring() {
    // Neither default_env.nu nor user's env.nu should be loaded
    // when using `nu -c`
    let nu = nu_test_support::fs::executable_path().display().to_string();
    let cmd = format!(
        r#"
            {nu} -c "view files | where filename =~ 'env\\.nu$' | length"
        "#
    );
    let actual = nu!(cmd);

    assert_eq!(actual.out, "0");
}
