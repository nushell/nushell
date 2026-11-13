use crate::repl::tests::{TestResult, fail_test, run_test, run_test_std};

/// Defaults for color_config, menus, and Nushell menu keybindings live on
/// `Config::default()` so they are visible even under `nu -n` / without sourcing
/// config files.
#[test]
fn default_config_color_menus_keybindings_viewable() -> TestResult {
    run_test(
        r#"
            [
                (($env.config.color_config | columns | length) > 0)
                ($env.config.color_config.header == "green_bold")
                ("completion_menu" in ($env.config.menus | get name))
                ("history_menu" in ($env.config.menus | get name))
                (($env.config.keybindings | length) > 0)
                ("completion_menu" in ($env.config.keybindings | get name))
                (($env.config.explore | columns | length) > 0)
                ("selected_cell" in ($env.config.explore | columns))
                ("title_bar" in ($env.config.tui | columns))
            ] | all {|x| $x }
        "#,
        "true",
    )
}

#[test]
fn mutate_nu_config() -> TestResult {
    run_test_std(
        "$env.config.footer_mode = 30; $env.config.footer_mode",
        "30",
    )
}

#[test]
fn mutate_nu_config_nested_ls() -> TestResult {
    run_test_std(
        "$env.config.ls.clickable_links = false; $env.config.ls.clickable_links",
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_table() -> TestResult {
    run_test_std(
        "
            $env.config.table.trim.methodology = 'wrapping'
            $env.config.table.trim.wrapping_try_keep_words = false
            $env.config.table.trim.wrapping_try_keep_words
        ",
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
                  type: { layout: columnar }
                  style: {}
                }
            ];
            $env.config.menus.0.type.columns = 3;
            $env.config.menus.0.type.columns
        "#,
        "3",
    )
}

/// `++=` appends an entry as written, an unnamed one included, and sourcing the
/// same config twice appends it twice: the list is a plain list.
#[test]
fn appending_keybindings_keeps_entries_as_written() -> TestResult {
    run_test_std(
        "
            let entry = { name: null, modifier: control, keycode: char_t, mode: emacs, event: null };
            let defaults = $env.config.keybindings | length;
            $env.config.keybindings ++= [$entry];
            $env.config.keybindings ++= [$entry];
            let last = $env.config.keybindings | last;
            let grew = ($env.config.keybindings | length) == $defaults + 2;
            $grew and $last.name == null and $last.keycode == 'char_t' and $last.event == null
        ",
        "true",
    )
}

/// Assigning `menus` replaces the list: `++=` keeps the defaults, a filtered
/// list drops what it dropped and `[]` clears it.
#[test]
fn assigning_menus_replaces_appends_and_filters() -> TestResult {
    run_test_std(
        r#"
            let defaults = $env.config.menus | length;
            $env.config.menus ++= [
                { name: extra, only_buffer_difference: false, marker: "E ", type: { layout: columnar }, style: {} }
            ];
            let appended = ($env.config.menus | length) == $defaults + 1;
            $env.config.menus = $env.config.menus | where name != extra;
            let filtered = ($env.config.menus | length) == $defaults;
            $env.config.menus = [];
            $appended and $filtered and ($env.config.menus | is-empty)
        "#,
        "true",
    )
}

#[test]
fn mutate_nu_config_nested_keybindings() -> TestResult {
    run_test_std(
        "
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
        ",
        "char_x",
    )
}

/// Assigning `keybindings` replaces the list: `++=` keeps the defaults, a
/// filtered list unbinds what it dropped and `[]` clears it.
#[test]
fn assigning_keybindings_replaces_appends_and_filters() -> TestResult {
    run_test_std(
        "
            let defaults = $env.config.keybindings | length;
            $env.config.keybindings ++= [
                { name: test, modifier: control, keycode: char_t, mode: [emacs vi_insert], event: null }
            ];
            let appended = ($env.config.keybindings | length) == $defaults + 1;
            $env.config.keybindings = $env.config.keybindings | where name != test;
            let filtered = ($env.config.keybindings | length) == $defaults;
            $env.config.keybindings = [];
            $appended and $filtered and ($env.config.keybindings | is-empty)
        ",
        "true",
    )
}

#[test]
fn mutate_nu_config_nested_color_nested() -> TestResult {
    run_test_std(
        "$env.config.color_config.shape_flag = 'cyan'; $env.config.color_config.shape_flag",
        "cyan",
    )
}

#[test]
fn mutate_nu_config_nested_completion() -> TestResult {
    run_test_std(
        "$env.config.completions.external.enable = false; $env.config.completions.external.enable",
        "false",
    )
}

#[test]
fn mutate_nu_config_nested_history() -> TestResult {
    run_test_std(
        "$env.config.history.max_size = 100; $env.config.history.max_size",
        "100",
    )
}

#[test]
fn mutate_nu_config_nested_filesize() -> TestResult {
    run_test_std(
        "$env.config.filesize.unit = 'kB'; $env.config.filesize.unit",
        "kB",
    )
}

#[test]
fn mutate_nu_config_plugin() -> TestResult {
    run_test_std(
        "
            $env.config.plugins = {
                config: {
                  key1: value
                  key2: other
                }
            };
            $env.config.plugins.config.key1 = 'updated'
            $env.config.plugins.config.key1
        ",
        "updated",
    )
}

#[test]
fn reject_nu_config_plugin_non_record() -> TestResult {
    fail_test("$env.config.plugins = 5", "Type mismatch")
}

#[test]
fn mutate_nu_config_plugin_gc_default_enabled() -> TestResult {
    run_test(
        "
            $env.config.plugin_gc.default.enabled = false
            $env.config.plugin_gc.default.enabled
        ",
        "false",
    )
}

#[test]
fn mutate_nu_config_plugin_gc_default_stop_after() -> TestResult {
    run_test(
        "
            $env.config.plugin_gc.default.stop_after = 20sec
            $env.config.plugin_gc.default.stop_after
        ",
        "20sec",
    )
}

#[test]
fn mutate_nu_config_plugin_gc_default_stop_after_negative() -> TestResult {
    fail_test(
        "
            $env.config.plugin_gc.default.stop_after = -1sec
            $env.config.plugin_gc.default.stop_after
        ",
        "expected a non-negative duration",
    )
}

#[test]
fn mutate_nu_config_plugin_gc_plugins() -> TestResult {
    run_test(
        "
            $env.config.plugin_gc.plugins.inc = {
                enabled: true
                stop_after: 0sec
            }
            $env.config.plugin_gc.plugins.inc.stop_after
        ",
        "0sec",
    )
}

#[test]
fn mutate_nu_config_history_warning() -> TestResult {
    fail_test(
        r#"$env.config.history.file_format = "plaintext"; $env.config.history.isolation = true"#,
        "history isolation only compatible with SQLite format",
    )
}

#[test]
fn mutate_nu_config_history_ignore_space() -> TestResult {
    run_test_std(
        "$env.config.history.ignore_space_prefixed = false; $env.config.history.ignore_space_prefixed",
        "false",
    )
}

/// A keybinding with a keycode that can't be read is rejected when it is
/// assigned, and the list stays as it was. This is the whole-record form,
/// which is how an older config may set it.
#[test]
fn a_record_with_a_keybinding_that_cannot_be_read_keeps_the_old_list() -> TestResult {
    run_test_std(
        "
            let before = $env.config.keybindings;
            let bad = { name: bad, modifier: control, keycode: nonsense, mode: emacs, event: null };
            let rejected = try { $env.config = ($env.config | upsert keybindings [$bad]); false } catch { true };
            $rejected and $env.config.keybindings == $before
        ",
        "true",
    )
}

/// The reload scenario: a broken entry is rejected, then the fixed one is added.
#[test]
fn a_fixed_keybinding_is_accepted_after_a_rejected_one() -> TestResult {
    run_test_std(
        "
            let n = $env.config.keybindings | length;
            let rejected = try { $env.config.keybindings ++= [{ name: mine, modifier: control, keycode: nonsense, mode: emacs, event: null }]; false } catch { true };
            $env.config.keybindings ++= [{ name: mine, modifier: control, keycode: char_t, mode: emacs, event: null }];
            $rejected and ($env.config.keybindings | length) == $n + 1
        ",
        "true",
    )
}

/// `config default` has the defaults, so `= []` is undone by assigning them again.
#[test]
fn config_default_restores_the_default_lists() -> TestResult {
    run_test_std(
        "
            let keybindings = $env.config.keybindings;
            let menus = $env.config.menus;
            $env.config.keybindings = [];
            $env.config.menus = [];
            $env.config.keybindings = (config default).keybindings;
            $env.config.menus = (config default).menus;
            $env.config.keybindings == $keybindings and $env.config.menus == $menus
        ",
        "true",
    )
}

/// Assigning the defaults first makes a config that is sourced again not grow the list.
#[test]
fn assigning_the_defaults_first_makes_appending_safe_to_repeat() -> TestResult {
    run_test_std(
        "
            let entry = { name: mine, modifier: control, keycode: char_t, mode: emacs, event: null };
            let defaults = (config default).keybindings | length;
            for _ in 1..3 {
                $env.config.keybindings = (config default).keybindings;
                $env.config.keybindings ++= [$entry]
            };
            ($env.config.keybindings | length) == $defaults + 1
        ",
        "true",
    )
}

#[test]
fn config_default_can_be_used_in_a_const() -> TestResult {
    run_test_std(
        "
            const menus = (config default | get menus | get name);
            $menus | first
        ",
        "completion_menu",
    )
}
