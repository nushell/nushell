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
fn mutate_nu_config_nested() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.ls.user_ls_colors = false; $env.config.ls.user_ls_colors"#,
        "false",
    )
}

#[test]
fn mutate_nu_config_nested2() -> TestResult {
    run_test_with_default_config(
        r#"$env.config.table.trim.wrapping_try_keep_words = false; $env.config.table.trim.wrapping_try_keep_words"#,
        "false",
    )
}
