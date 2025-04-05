use crate::repl::tests::{fail_test, run_test, run_test_std, TestResult};
use nu_test_support::{nu, nu_repl_code};

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
        r#"$env.config.filesize.unit = 'kB'; $env.config.filesize.unit"#,
        "kB",
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
fn config_is_mutable() {
    let actual = nu!(nu_repl_code(&[
        r"$env.config = { ls: { clickable_links: true } }",
        "$env.config.ls.clickable_links = false;",
        "$env.config.ls.clickable_links"
    ]));

    assert_eq!(actual.out, "false");
}

#[test]
fn config_preserved_after_do() {
    let actual = nu!(nu_repl_code(&[
        r"$env.config = { ls: { clickable_links: true } }",
        "do -i { $env.config.ls.clickable_links = false }",
        "$env.config.ls.clickable_links"
    ]));

    assert_eq!(actual.out, "true");
}

#[test]
fn config_affected_when_mutated() {
    let actual = nu!(nu_repl_code(&[
        r#"$env.config = { filesize: { unit: binary } }"#,
        r#"$env.config = { filesize: { unit: metric } }"#,
        "20MB | into string"
    ]));

    assert_eq!(actual.out, "20.0 MB");
}

#[test]
fn config_affected_when_deep_mutated() {
    let actual = nu!(cwd: "crates/nu-utils/src/default_files", nu_repl_code(&[
        r#"source default_config.nu"#,
        r#"$env.config.filesize.unit = 'binary'"#,
        r#"20MiB | into string"#]));

    assert_eq!(actual.out, "20.0 MiB");
}

#[test]
fn config_add_unsupported_key() {
    let actual = nu!(cwd: "crates/nu-utils/src/default_files", nu_repl_code(&[
        r#"source default_config.nu"#,
        r#"$env.config.foo = 2"#,
        r#";"#]));

    assert!(actual
        .err
        .contains("Unknown config option: $env.config.foo"));
}

#[test]
fn config_add_unsupported_type() {
    let actual = nu!(cwd: "crates/nu-utils/src/default_files", nu_repl_code(&[r#"source default_config.nu"#,
        r#"$env.config.ls = '' "#,
        r#";"#]));

    assert!(actual.err.contains("Type mismatch"));
}

#[test]
fn config_add_unsupported_value() {
    let actual = nu!(cwd: "crates/nu-utils/src/default_files", nu_repl_code(&[r#"source default_config.nu"#,
        r#"$env.config.history.file_format = ''"#,
        r#";"#]));

    assert!(actual.err.contains("Invalid value"));
    assert!(actual.err.contains("expected 'sqlite' or 'plaintext'"));
}

#[test]
#[ignore = "Figure out how to make test_bins::nu_repl() continue execution after shell errors"]
fn config_unsupported_key_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/default_files", nu_repl_code(&[r#"source default_config.nu"#,
        r#"$env.config.foo = 1"#,
        r#"'foo' in $env.config"#]));

    assert_eq!(actual.out, "false");
}

#[test]
#[ignore = "Figure out how to make test_bins::nu_repl() continue execution after shell errors"]
fn config_unsupported_type_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/default_files", nu_repl_code(&[r#" source default_config.nu"#,
        r#"$env.config.ls = ''"#,
        r#"$env.config.ls | describe"#]));

    assert_eq!(actual.out, "record");
}

#[test]
#[ignore = "Figure out how to make test_bins::nu_repl() continue execution after errors"]
fn config_unsupported_value_reverted() {
    let actual = nu!(cwd: "crates/nu-utils/src/default_files", nu_repl_code(&[r#" source default_config.nu"#,
        r#"$env.config.history.file_format = 'plaintext'"#,
        r#"$env.config.history.file_format = ''"#,
        r#"$env.config.history.file_format | to json"#]));

    assert_eq!(actual.out, "\"plaintext\"");
}

#[test]
fn filesize_mb() {
    let code = &[
        r#"$env.config = { filesize: { unit: MB } }"#,
        r#"20MB | into string"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, "20.0 MB");
}

#[test]
fn filesize_mib() {
    let code = &[
        r#"$env.config = { filesize: { unit: MiB } }"#,
        r#"20MiB | into string"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, "20.0 MiB");
}

#[test]
fn filesize_format_decimal() {
    let code = &[
        r#"$env.config = { filesize: { unit: metric } }"#,
        r#"[2MB 2GB 2TB] | into string | to nuon"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, r#"["2.0 MB", "2.0 GB", "2.0 TB"]"#);
}

#[test]
fn filesize_format_binary() {
    let code = &[
        r#"$env.config = { filesize: { unit: binary } }"#,
        r#"[2MiB 2GiB 2TiB] | into string | to nuon"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, r#"["2.0 MiB", "2.0 GiB", "2.0 TiB"]"#);
}

#[test]
fn fancy_default_errors() {
    let code = nu_repl_code(&[
        "$env.config.use_ansi_coloring = true",
        r#"def force_error [x] {
        error make {
            msg: "oh no!"
            label: {
                text: "here's the error"
                span: (metadata $x).span
            }
        }
    }"#,
        r#"force_error "My error""#,
    ]);

    let actual = nu!(format!("try {{ {code} }}"));

    assert_eq!(
        actual.err,
        "Error:   \u{1b}[31m×\u{1b}[0m oh no!\n   ╭─[\u{1b}[36;1;4mline2:1:13\u{1b}[0m]\n \u{1b}[2m1\u{1b}[0m │ force_error \"My error\"\n   · \u{1b}[35;1m            ─────┬────\u{1b}[0m\n   ·                  \u{1b}[35;1m╰── \u{1b}[35;1mhere's the error\u{1b}[0m\u{1b}[0m\n   ╰────\n\n"
    );
}

#[test]
fn narratable_errors() {
    let code = nu_repl_code(&[
        r#"$env.config = { error_style: "plain" }"#,
        r#"def force_error [x] {
        error make {
            msg: "oh no!"
            label: {
                text: "here's the error"
                span: (metadata $x).span
            }
        }
    }"#,
        r#"force_error "my error""#,
    ]);

    let actual = nu!(format!("try {{ {code} }}"));

    assert_eq!(
        actual.err,
        r#"Error: oh no!
    Diagnostic severity: error
Begin snippet for line2 starting at line 1, column 1

snippet line 1: force_error "my error"
    label at line 1, columns 13 to 22: here's the error


"#,
    );
}

#[test]
fn plugins() {
    let code = &[
        r#"$env.config = { plugins: { nu_plugin_config: { key: value } } }"#,
        r#"$env.config.plugins"#,
    ];
    let actual = nu!(nu_repl_code(code));
    assert_eq!(actual.out, r#"{nu_plugin_config: {key: value}}"#);
}
