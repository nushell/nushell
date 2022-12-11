use crate::tests::{run_test, TestResult};

use super::run_test_with_default_config;

#[test]
fn mutate_nu_config() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.footer_mode = 30; $env.config.footer_mode"#,
        "30",
    )
}

#[test]
fn mutate_nu_config_nested_ls() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.ls.user_ls_colors = false; $env.config.ls.user_ls_colors"#,
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_table() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.table.trim.wrapping_try_keep_words = false; $env.config.table.trim.wrapping_try_keep_words"#,
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_menu() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.menu.2.type.columns = 3; $env.config.menu.2.type.columns"#,
        "3",
    )
}

#[test]
fn mutate_nu_config_nested_keybindings() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.keybindings.5.keycode = 'char_x'; $env.config.keybindings.5.keycode"#,
        "char_x",
    )
}

#[test]
fn mutate_nu_config_nested_color_nested() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.color_config.shape_flag = 'cyan'; $env.config.color_config.shape_flag"#,
        "cyan",
    )
}

#[test]
fn mutate_nu_config_nested_completion() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.completions.external.enable = false; $env.config.completions.external.enable"#,
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_history() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.history.max_size = 100; $env.config.history.max_size"#,
        "100",
    )
}

#[test]
fn mutate_nu_config_nested_filesize() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.filesize.format = 'kb'; $env.config.filesize.format"#,
        "kb",
    )
}
