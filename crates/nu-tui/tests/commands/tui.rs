use nu_protocol::ConfigError;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn parent_command_lists_subcommands() -> Result {
    let help: String = test().run("tui")?;
    for sub in [
        "tui run",
        "tui debug",
        "tui bind",
        "tui theme",
        "tui context",
        "tui table",
        "tui search",
        "tui split",
        "tui box",
        "tui tab",
        "tui tree",
        "tui select",
        "tui button",
        "tui progress",
    ] {
        assert_contains(sub, &help);
    }
    Ok(())
}

#[test]
fn theme_slots_ride_in_the_tui_value_and_unknown_ones_are_errors() -> Result {
    let bg: String = test().run(
        "tui table | tui theme {selected: {bg: red}} | tui theme {file_styles: [{kind: dir, fg: cyan}]} | get theme.selected.bg",
    )?;
    assert_eq!(bg, "red");
    let err = test()
        .run("tui table | tui theme {not_a_slot: red}")
        .expect_error()?;
    assert_contains("not_a_slot", &format!("{err:?}"));
    assert_contains("file_styles", &format!("{err:?}"));
    let err = test()
        .run("tui table | tui theme {file_styles: [{kind: folder}]}")
        .expect_error()?;
    assert_contains("symlink", &format!("{err:?}"));
    let err = test()
        .run("tui table | tui theme {border_type: wavy}")
        .expect_error()?;
    assert_contains("wavy", &format!("{err:?}"));
    Ok(())
}

#[test]
fn glyph_slots_shape_tabs_marks_caps_and_the_gauge() -> Result {
    let screen: String = test().run(
        r#"[a b] | tui label --titlebar app | tui tab one [(tui select --multi)] | tui tab two [(tui table)]
            | tui theme {title_caps: {open: "<", close: ">"}, tab_caps: {open: "[", close: "]"}, selected_caps: {open: "(", close: ")"}, check: {on: "*", off: "-"}}
            | tui debug --size [40 8] | get screen"#,
    )?;
    assert_contains("< app >", &screen);
    assert_contains("[ one ]", &screen);
    assert_contains("[ two ]", &screen);
    // The highlighted row sits between its caps; the other row has none.
    assert_contains("(- a", &screen);
    assert_contains(")", &screen);
    assert!(!screen.contains("(- b"), "{screen}");
    let screen: String = test().run(
        r#"tui progress --value 0.5 | tui theme {progress_fill: "="} | tui debug --size [20 3] | get screen"#,
    )?;
    assert_contains("====", &screen);
    let screen: String = test().run(
        r#"[a b] | tui select | tui theme {radio: {on: ">>", off: "  "}} | tui debug --size [20 5] | get screen"#,
    )?;
    assert_contains(">> a", &screen);
    assert_contains("   b", &screen);
    let err = test()
        .run("tui table | tui theme {check: {on: x, off: oo}}")
        .expect_error()?;
    assert_contains("one width", &format!("{err:?}"));
    let err = test()
        .run("tui table | tui theme {progress_fill: ab}")
        .expect_error()?;
    assert_contains("1 cell wide", &format!("{err:?}"));
    Ok(())
}

#[test]
fn headless_renders_title_and_status() -> Result {
    let screen: String = test().run(
        r#"tui label --titlebar "Demo" | tui label --status "ready" | tui debug | get screen"#,
    )?;
    assert_contains("Demo", &screen);
    assert_contains("ready", &screen);
    Ok(())
}

#[test]
fn title_replaces_the_widget_name_and_keeps_its_state() -> Result {
    // Each widget is the first in its column, so its border starts with `┌`
    // and the title is followed by the border line. That pins the whole
    // title: `┌ source ─` fails on `┌ source (preview) ─`.
    let cases = [
        ("[a b c] | tui table --title files", "files (3)"),
        (
            "[a b c] | tui table --multi --title files | tui debug --keys [space] --size [40 8] | get screen",
            "files (3, 1 checked)",
        ),
        ("[a b] | tui select --title pick", "pick (2)"),
        ("[a b] | tui tree --title nodes", "nodes (2)"),
        ("[x] | tui log --title events", "events"),
        (
            "[x] | tui search --fuzzy --title find | tui table",
            "find (fuzzy)",
        ),
        ("tui textbox --title name", "name"),
        ("tui preview --title source", "source"),
        // A row without a `name` column: nothing to preview.
        (
            "[{a: 1}] | tui table | tui preview --title source",
            "source",
        ),
    ];
    for (code, expected) in cases {
        let code = if code.contains("tui debug") {
            code.to_string()
        } else {
            format!("{code} | tui debug --size [40 8] | get screen")
        };
        let screen: String = test().run(&code)?;
        assert_contains(format!("┌ {expected} ─"), &screen);
    }
    let title: String = test().run("tui table --title files | get widgets.0.title")?;
    assert_eq!(title, "files");
    Ok(())
}

#[test]
fn title_must_not_be_empty() -> Result {
    let err = test()
        .run(r#"[a b] | tui table --title """#)
        .expect_error()?;
    assert_contains("empty --title", &format!("{err:?}"));
    Ok(())
}

#[test]
fn search_placeholder_defaults_to_the_title() -> Result {
    // The table takes focus, so the empty search box shows its placeholder.
    let screen: String = test()
        .run("[x] | tui search --title find | tui table | tui debug --size [40 8] | get screen")?;
    assert_contains("┌ find ─", &screen);
    assert_contains("│find ", &screen);
    let placeholder: String =
        test().run("[x] | tui search --title find --placeholder q | get widgets.0.placeholder")?;
    assert_eq!(placeholder, "q");
    Ok(())
}

#[test]
fn label_title_bar_switch_is_titlebar() -> Result {
    let bar: String = test().run(r#"tui label --titlebar "App" | get widgets.0.slot"#)?;
    assert_eq!(bar, "titlebar");
    // `--title` is the deprecated 0.116.0 name: it still works, and is
    // listed as deprecated.
    let bar: String = test().run(r#"tui label --title "App" | get widgets.0.slot"#)?;
    assert_eq!(bar, "titlebar");
    let bar: String = test().run(r#"tui label --title=true "App" | get widgets.0.slot"#)?;
    assert_eq!(bar, "titlebar");
    let columns: Vec<String> =
        test().run(r#"tui label --title "App" | get widgets.0 | columns"#)?;
    assert!(!columns.iter().any(|c| c == "title"), "got {columns:?}");
    let deprecated: Vec<String> = test()
        .run(r#"scope commands | where name == "tui label" | get 0.deprecation_info.flag"#)?;
    assert_eq!(deprecated, vec!["title".to_string()]);
    Ok(())
}

#[test]
fn border_draws_the_named_table_theme_outline() -> Result {
    let cases = [
        ("[a] | tui table --border double", "╔"),
        ("[a] | tui table --border Rounded", "╭"),
        ("[a] | tui log --border heavy", "┏"),
        ("tui textbox --border basic", "+"),
        ("tui box Group [(tui textbox)] --border ascii_rounded", "'"),
        (
            "$env.config.tui.border_type = 'double'; [a] | tui table",
            "╔",
        ),
        (
            "$env.config.tui.border_type = 'double'; [a] | tui table --border rounded",
            "╭",
        ),
    ];
    for (code, corner) in cases {
        let screen: String =
            test().run(format!("{code} | tui debug --size [30 6] | get screen"))?;
        assert_contains(corner, &screen);
    }
    Ok(())
}

#[test]
fn menu_border_draws_its_dropdown() -> Result {
    let code = r#"
        tui menu [{name: "&File", items: ["&Open"]}] --border double
        | tui debug --size [30 6] --keys "alt+f"
        | get screen
    "#;
    let screen: String = test().run(code)?;
    assert_contains("╔", &screen);
    Ok(())
}

#[test]
fn widget_record_shows_its_border() -> Result {
    test()
        .run("[a] | tui table --border with_love | tui debug | get widgets.0.border")
        .expect_value_eq("with_love")
}

#[test]
fn frameless_border_keeps_its_cell_and_title() -> Result {
    let screen: String = test()
        .run("[a b] | tui table --border frameless | tui debug --size [20 5] | get screen")?;
    assert_contains_not("─", &screen);
    assert_contains_not("│", &screen);
    // The blank border still takes the first column: every row starts
    // blank, and the padded title ` table (2) ` starts after the corner.
    // Screen lines are trimmed, so the blank bottom border is empty.
    for line in screen.lines().filter(|line| !line.is_empty()) {
        assert!(line.starts_with(' '), "{line:?} in\n{screen}");
    }
    assert!(screen.starts_with("  table (2)"), "{screen}");
    Ok(())
}

#[test]
fn none_default_and_unknown_borders_are_rejected() -> Result {
    let err = test()
        .run("[a] | tui table --border none")
        .expect_shell_error()?;
    assert_contains("`none` is not a tui border", err.generic_error()?);
    for name in ["default", "wavy"] {
        let err = test()
            .run(format!("[a] | tui table --border {name}"))
            .expect_shell_error()?;
        assert!(
            matches!(&err, ShellError::InvalidValue { actual, valid, .. }
                if actual == name && valid.contains("frameless") && !valid.contains("none")),
            "{err:?}"
        );
    }
    Ok(())
}

#[test]
fn config_rejects_a_non_border_and_keeps_the_old_one() -> Result {
    for name in ["none", "default", "wavy"] {
        let mut tester = test();
        let () = tester.run("$env.config.tui.border_type = 'double'")?;
        let err = tester
            .run(format!("$env.config.tui.border_type = '{name}'"))
            .expect_shell_error()?;
        assert!(
            matches!(&err, ShellError::InvalidConfig { errors } if matches!(
                errors.as_slice(),
                [ConfigError::InvalidValue { path, .. }] if path == "$env.config.tui.border_type"
            )),
            "{err:?}"
        );
        tester
            .run("$env.config.tui.border_type")
            .expect_value_eq("double")?;
    }
    Ok(())
}

#[test]
fn table_keys_submit_selected_row() -> Result {
    let code = "
        [{name: alpha, n: 1}, {name: beta, n: 2}, {name: gamma, n: 3}]
        | tui table --columns [name n]
        | tui debug --keys down,enter
    ";
    let action: String = test().run(format!("{code} | get action"))?;
    assert_eq!(action, "submit");
    let name: String = test().run(format!("{code} | get selected.name"))?;
    assert_eq!(name, "beta");
    Ok(())
}

#[test]
fn keys_accept_a_list_of_tokens() -> Result {
    let name: String = test().run(
        "[{name: a}, {name: b}, {name: c}] | tui table | tui debug --keys [down down enter] | get selected.name",
    )?;
    assert_eq!(name, "c");
    Ok(())
}

#[test]
fn until_stops_the_replay_early() -> Result {
    let index: i64 = test().run(
        "[a b c d] | tui table | tui debug --keys [down down down] --until {|s| $s.values.table-0.index == 1 } | get values.table-0.index",
    )?;
    assert_eq!(index, 1);
    Ok(())
}

#[test]
fn values_hold_every_widget_by_id() -> Result {
    let values: String = test().run(
        r#"tui textbox --id name | tui search --id q | tui debug --keys "type:hi" | get values | to nuon"#,
    )?;
    assert_contains("name: hi", &values);
    assert_contains(r#"q: """#, &values);
    let index: i64 = test()
        .run("[a b c] | tui table --id t | tui debug --keys [down down] | get values.t.index")?;
    assert_eq!(index, 2);
    Ok(())
}

#[test]
fn search_filters_table_before_submit() -> Result {
    let code = r#"
        [{name: alpha}, {name: beta}, {name: gamma}]
        | tui search --placeholder "filter" --bind /
        | tui table --columns [name]
        | tui debug --keys "/,type:ga,tab,enter"
    "#;
    let name: String = test().run(format!("{code} | get selected.name"))?;
    assert_eq!(name, "gamma");
    Ok(())
}

#[test]
fn fuzzy_search_matches_a_subsequence_on_one_column() -> Result {
    let name: String = test().run(
        r#"
            [{name: alpha, tag: gm}, {name: gamma, tag: x}]
            | tui search --fuzzy --columns [name] --bind /
            | tui table
            | tui debug --keys "/,type:gm,enter"
            | get selected.name
        "#,
    )?;
    assert_eq!(name, "gamma");
    let rows: i64 = test().run(
        r#"
            [{name: alpha, tag: gm}, {name: gamma, tag: x}]
            | tui search --columns [name] --bind /
            | tui table
            | tui debug --keys "/,type:gm"
            | get widgets.1.rows
        "#,
    )?;
    assert_eq!(rows, 0, "substring on the name column only");
    Ok(())
}

#[test]
fn case_sensitive_search_respects_case() -> Result {
    let rows: i64 = test().run(
        r#"[{name: Alpha}] | tui search --case-sensitive --bind / | tui table | tui debug --keys "/,type:alpha" | get widgets.1.rows"#,
    )?;
    assert_eq!(rows, 0);
    let rows: i64 = test().run(
        r#"[{name: Alpha}] | tui search --bind / | tui table | tui debug --keys "/,type:alpha" | get widgets.1.rows"#,
    )?;
    assert_eq!(rows, 1);
    Ok(())
}

#[test]
fn nested_search_only_filters_its_container() -> Result {
    let rows: Vec<i64> = test().run(
        r#"
            [alpha beta]
            | tui split [
                (tui split --vertical [(tui search --bind /) (tui table --id a)])
                (tui table --id b)
              ]
            | tui debug --keys "/,type:zzz"
            | get widgets.0.children
            | [($in.0.children.1.rows) ($in.1.rows)]
        "#,
    )?;
    assert_eq!(rows, vec![0, 2]);
    Ok(())
}

#[test]
fn textbox_typing_does_not_quit_on_q() -> Result {
    let code = r#"
        tui textbox --placeholder "name"
        | tui debug --keys "type:q,enter"
    "#;
    let action: String = test().run(format!("{code} | get action"))?;
    assert_eq!(action, "submit");
    let selected: String = test().run(format!("{code} | get selected"))?;
    assert_eq!(selected, "q");
    Ok(())
}

#[test]
fn q_quits_table_without_submit() -> Result {
    let action: String = test().run("[{name: a}] | tui table | tui debug --keys q | get action")?;
    assert_eq!(action, "quit");
    Ok(())
}

#[test]
fn pipeline_app_is_custom_tui_value() -> Result {
    let desc: String = test().run(r#"tui label --titlebar "x" | describe"#)?;
    assert_contains("tui", &desc);
    Ok(())
}

#[test]
fn scalar_rows_show_as_item_column() -> Result {
    let screen: String = test().run("[one two] | tui table | tui debug | get screen")?;
    assert_contains("item", &screen);
    assert_contains("two", &screen);
    Ok(())
}

#[test]
fn capture_keys_chord_selects_matching_row() -> Result {
    let name: String = test().run(
        r#"
            [
                {name: "history", modifier: "control", keycode: "char_r", mode: "emacs"}
                {name: "clear", modifier: "control", keycode: "char_l", mode: "emacs"}
            ]
            | tui table --capture-keys
            | tui debug --keys "ctrl+r,enter"
            | get selected.name
        "#,
    )?;
    assert_eq!(name, "history");
    Ok(())
}

#[test]
fn tab_pages_switch_with_bracket_and_digit() -> Result {
    let screen: String = test().run(
        r#"
            tui tab "one" [(tui label "PAGE-ONE")]
            | tui tab "two" [(tui label "PAGE-TWO")]
            | tui debug --keys "]"
            | get screen
        "#,
    )?;
    assert_contains("PAGE-TWO", &screen);
    assert_contains(" one ", &screen);
    let page: i64 = test().run(
        r#"
            tui tab "one" [(tui label "PAGE-ONE")]
            | tui tab "two" [(tui label "PAGE-TWO")]
            | tui debug --keys "2"
            | get page
        "#,
    )?;
    assert_eq!(page, 1);
    Ok(())
}

#[test]
fn single_tab_still_shows_the_bar() -> Result {
    let pages: Vec<String> =
        test().run(r#"tui tab "only" [(tui label "x")] | tui debug | get pages.title"#)?;
    assert_eq!(pages, vec!["only".to_string()]);
    Ok(())
}

#[test]
fn box_is_a_bordered_group_and_tabs_cannot_nest() -> Result {
    let screen: String = test().run(
        r#"
            tui split [(tui box "left" [(tui label "A")]) (tui label "B")]
            | tui debug --size [40 8]
            | get screen
        "#,
    )?;
    assert_contains(" left ", &screen);
    assert_contains("┌", &screen);
    let err = test()
        .run(r#"tui split [(tui tab "left" [(tui label "A")]) (tui label "B")] | tui debug"#)
        .expect_error()?;
    assert_contains("tui box", &format!("{err:?}"));
    Ok(())
}

#[test]
fn preview_shows_file_contents_on_selection() -> Result {
    Playground::setup("tui_preview", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("one.txt", "ONE-FILE-BODY"),
            FileWithContent("two.txt", "TWO-FILE-BODY"),
        ]);

        let screen: String = test().cwd(dirs.test()).run(
            "
                ls
                | sort-by name
                | tui split [(tui table --columns [name type]) (tui preview)]
                | tui debug --size [80 16]
                | get screen
            ",
        )?;
        assert_contains("ONE-FILE-BODY", &screen);

        let screen: String = test().cwd(dirs.test()).run(
            "
                ls
                | sort-by name
                | tui split [(tui table --columns [name type]) (tui preview)]
                | tui debug --keys down --size [80 16]
                | get screen
            ",
        )?;
        assert_contains("TWO-FILE-BODY", &screen);
        Ok(())
    })
}

#[test]
fn preview_closure_transforms_file_contents() -> Result {
    Playground::setup("tui_preview_closure", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("note.txt", "hello-preview")]);

        let screen: String = test().cwd(dirs.test()).run(
            r#"
                [{name: "note.txt", type: "file"}]
                | tui split [(tui table) (tui preview { $in | str upcase })]
                | tui debug --size [80 16]
                | get screen
            "#,
        )?;
        assert_contains("HELLO-PREVIEW", &screen);
        Ok(())
    })
}

#[test]
fn preview_closure_with_row_param_is_the_source() -> Result {
    let screen: String = test().run(
        r#"
            [{name: "no-such-file", event: {send: "OpenHistory"}}]
            | tui split [(tui table --columns [name]) (tui preview {|row| $row.event | to nuon })]
            | tui debug --size [80 16]
            | get screen
        "#,
    )?;
    assert_contains("OpenHistory", &screen);
    Ok(())
}

#[test]
fn builders_do_not_read_their_stream() -> Result {
    // Reading even one row of this stream takes a second.
    let fast: bool = test().run(
        "(timeit { 1.. | each {|n| sleep 1sec; $n } | tui label --titlebar x | tui log | ignore }) < 1sec",
    )?;
    assert!(fast, "a builder waited on the stream");
    Ok(())
}

#[test]
fn slow_streams_are_live_and_child_streams_start_empty() -> Result {
    // The outer stream feeds the log as it ticks. The table's own stream
    // has produced nothing yet, so the table must be empty rather than
    // showing the outer rows.
    let checks: Vec<bool> = test().run(
        r#"
            1.. | each {|n| sleep 50ms; $"tick ($n)" }
            | tui split [(tui log) (1.. | each {|n| sleep 10sec; $n } | tui table)]
            | tui debug --size [60 8]
            | [$in.live ($in.screen | str contains "tick ") ($in.widgets.0.children.1.rows == 0)]
        "#,
    )?;
    assert_eq!(checks, vec![true, true, true]);
    Ok(())
}

#[test]
fn slow_child_lists_are_read_in_full() -> Result {
    let rows: i64 = test().run(
        "tui split [(1..3 | each { sleep 150ms; {name: $in} } | tui table)] | tui debug | get widgets.0.children.0.rows",
    )?;
    assert_eq!(rows, 3);
    Ok(())
}

#[test]
fn a_saved_tui_value_shows_its_stream_on_every_run() -> Result {
    let rows: Vec<i64> = test().run(
        "let t = (1..3 | each { $in } | tui table); [($t | tui debug) ($t | tui debug)] | each { $in.widgets.0.rows }",
    )?;
    assert_eq!(rows, vec![3, 3]);
    Ok(())
}

#[test]
fn debug_keeps_the_newest_rows_of_a_long_stream() -> Result {
    // Read to the end, as `tui run` would: the newest 100k rows, starting
    // at 50001, and no longer live.
    let checks: Vec<bool> = test().run(
        r#"
            1..150000 | tui table | tui debug --size [30 6]
            | [($in.widgets.0.rows == 100000) ($in.screen | str contains "50001") (not $in.live)]
        "#,
    )?;
    assert_eq!(checks, vec![true, true, true]);
    Ok(())
}

#[test]
fn one_stream_shown_twice_fills_both_widgets() -> Result {
    let rows: Vec<i64> = test().run(
        "let t = (1..3 | each { $in } | tui table); $t | tui split [$t] | tui debug | [$in.widgets.0.rows $in.widgets.1.children.0.rows]",
    )?;
    assert_eq!(rows, vec![3, 3]);
    Ok(())
}

#[test]
fn piped_data_no_widget_can_show_is_an_error() -> Result {
    let err = test()
        .run("tui split [(1..3 | each { $in } | tui table --data [])] | tui debug")
        .expect_error()?;
    assert_contains("no widget to show it", &format!("{err:?}"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn closing_the_tui_stops_an_external_producer() -> Result {
    // `tail -f` never exits on its own; `tui debug` must still return.
    // The test shell has no PATH; `tail` lives here on macOS and Linux.
    let live: bool = test().run("^/usr/bin/tail -f /dev/null | tui log | tui debug | get live")?;
    assert!(live);
    Ok(())
}

#[test]
fn streamed_rows_reach_the_tui() -> Result {
    let screen: String = test().run(
        r#"
            1..5
            | each {|n| sleep 1ms; $n}
            | tui label --titlebar "stream"
            | tui table
            | tui debug --size [40 12]
            | get screen
        "#,
    )?;
    assert_contains("5", &screen);
    assert_contains("stream", &screen);
    Ok(())
}

#[test]
fn dialog_headless_draws_close_control() -> Result {
    let screen: String = test().run(
        r#"tui label --titlebar "popup" | tui label --status "hi" | tui debug --dialog --size [80 24] | get screen"#,
    )?;
    assert_contains("popup", &screen);
    assert_contains("x", &screen);
    Ok(())
}

#[test]
fn menu_items_reject_non_strings() -> Result {
    test()
        .run("tui menu {a: 1} | tui debug")
        .expect_parse_error()?;
    Ok(())
}

#[test]
fn widgets_field_is_inspectable() -> Result {
    let slots: Vec<String> =
        test().run(r#"tui label --titlebar "App" | tui label --status "ok" | get widgets.slot"#)?;
    assert_eq!(slots, vec!["titlebar".to_string(), "status".to_string()]);
    let types: Vec<String> =
        test().run("tui split [(tui table) (tui preview)] | get widgets.0.children.type")?;
    assert_eq!(types, vec!["table".to_string(), "preview".to_string()]);
    Ok(())
}

#[test]
fn debug_reports_layout_rects_and_focus() -> Result {
    let widths: Vec<i64> = test().run(
        "
            [{name: a}]
            | tui split --ratio 60 [(tui table) (tui preview)]
            | tui debug --size [80 24]
            | get widgets.0.children.rect.width
        ",
    )?;
    assert_eq!(widths.len(), 2);
    assert!(widths[0] > widths[1], "60/40 split, got {widths:?}");
    let focus: String = test().run(
        "[{name: a}] | tui split [(tui table) (tui preview)] | tui debug | get focus.default",
    )?;
    assert_eq!(focus, "table-0");
    let source: String = test().run(
        "[{name: a}] | tui split [(tui table) (tui preview)] | tui debug | get widgets.0.children.1.source",
    )?;
    assert_eq!(source, "table-0");
    Ok(())
}

#[test]
fn split_sizes_take_cells_percent_and_fractions() -> Result {
    let widths: Vec<i64> = test().run(
        r#"
            tui split --sizes [20 "25%" 1fr] [(tui label a) (tui label b) (tui label c)]
            | tui debug --size [82 8]
            | get widgets.0.children.rect.width
        "#,
    )?;
    assert_eq!(widths[0], 20);
    assert!(
        (20..=21).contains(&widths[1]),
        "25% of the split: {widths:?}"
    );
    assert!(widths[2] >= 38, "the fraction takes the rest: {widths:?}");
    let err = test()
        .run("tui split --sizes [wide] [(tui label a)] | tui debug")
        .expect_error()?;
    assert_contains("size", &format!("{err:?}"));
    Ok(())
}

#[test]
fn tree_expands_nested_record() -> Result {
    let screen: String = test().run(
        r#"
            {a: {b: 1, c: 2}, d: [3, 4]}
            | tui tree
            | tui debug --keys "right,down,right,down,right" --size [40 16]
            | get screen
        "#,
    )?;
    assert_contains("a", &screen);
    assert_contains("b: 1", &screen);
    assert_contains("c: 2", &screen);
    assert_eq!(screen.matches("value").count(), 0);
    Ok(())
}

#[test]
fn split_renders_both_children_side_by_side() -> Result {
    let screen: String = test().run(
        r#"
            tui split [(tui label "LEFT") (tui label "RIGHT")]
            | tui debug --size [40 8]
            | get screen
        "#,
    )?;
    let line = screen
        .lines()
        .find(|l| l.contains("LEFT"))
        .expect("a line with LEFT");
    assert_contains("RIGHT", line);
    Ok(())
}

#[test]
fn split_children_keep_their_own_data() -> Result {
    let names: Vec<String> = test().run(
        "
            tui split [([{name: x}] | tui table) ([{name: y} {name: z}] | tui table)]
            | tui debug
            | [$in.values.table-0.row.name $in.values.table-1.row.name]
        ",
    )?;
    assert_eq!(names, vec!["x".to_string(), "y".to_string()]);
    let rows: Vec<i64> = test().run(
        "tui split [(ls crates | tui table) (ls crates/nu-tui | tui table)] | tui debug | get widgets.0.children.rows",
    )?;
    assert!(
        rows[0] > rows[1] && rows[1] > 0,
        "streams that finish are collected: {rows:?}"
    );
    let rows: Vec<i64> = test().run(
        "
            [shared]
            | tui split [(tui table) (tui table --data [a b c])]
            | tui debug
            | get widgets.0.children.rows
        ",
    )?;
    assert_eq!(rows, vec![1, 3]);
    Ok(())
}

#[test]
fn from_closure_makes_a_detail_view() -> Result {
    let rows: i64 = test().run(
        "
            [{name: a, kids: [1 2]}, {name: b, kids: [3 4 5]}]
            | tui split [(tui table --id src --columns [name]) (tui table --from src {|r| $r.kids })]
            | tui debug --keys [down]
            | get widgets.0.children.1.rows
        ",
    )?;
    assert_eq!(rows, 3);
    let screen: String = test().run(
        r#"
            [{name: a, size: 10}, {name: b, size: 20}]
            | tui table --columns [name]
            | tui label {|row| $"SIZE-($row.size)" }
            | tui debug --keys [down] --size [40 10]
            | get screen
        "#,
    )?;
    assert_contains("SIZE-20", &screen);
    Ok(())
}

#[test]
fn split_renumbers_colliding_auto_ids() -> Result {
    let ids: Vec<String> = test()
        .run("[a] | tui split [(tui table) (tui table)] | tui debug | get widgets.0.children.id")?;
    assert_eq!(ids, vec!["table-0".to_string(), "table-1".to_string()]);
    let source: String = test().run(
        "[a] | tui split [(tui table) (tui split [(tui table) (tui preview --from table-0)])] | tui debug | get widgets.0.children.1.children.1.source",
    )?;
    assert_eq!(source, "table-1");
    Ok(())
}

#[test]
fn split_rejects_duplicate_explicit_ids_and_nested_chrome() -> Result {
    let err = test()
        .run("tui split [(tui table --id x) (tui table --id x)] | tui debug")
        .expect_error()?;
    assert_contains("duplicate", &format!("{err:?}"));
    let err = test()
        .run(r#"tui split [(tui label --titlebar "x")] | tui debug"#)
        .expect_error()?;
    assert_contains("chrome", &format!("{err:?}"));
    Ok(())
}

#[test]
fn range_input_expands_to_rows() -> Result {
    let rows: i64 = test().run("1..20 | tui table | tui debug | get rows")?;
    assert_eq!(rows, 20);
    Ok(())
}

#[test]
fn menu_alt_mnemonic_submits_bar_item() -> Result {
    let selected: String = test().run(
        r#"tui menu ["&File" "&Edit" "&View"] | tui table | tui debug --keys "alt+e,enter" | get selected"#,
    )?;
    assert_eq!(selected, "Edit");
    Ok(())
}

#[test]
fn menu_dropdown_item_submits_record() -> Result {
    let item: String = test().run(
        r#"
            [a b]
            | tui menu [{name: "&File", items: ["&Open" "&Quit"]}]
            | tui table
            | tui debug --keys "alt+f,q"
            | get selected.item
        "#,
    )?;
    assert_eq!(item, "Quit");
    let row: String = test().run(
        r#"
            [a b]
            | tui menu [{name: "&File", items: ["&Open" "&Quit"]}]
            | tui table
            | tui debug --keys "down,alt+f,o"
            | get selected.row
        "#,
    )?;
    assert_eq!(row, "b");
    Ok(())
}

#[test]
fn menu_action_is_a_hook() -> Result {
    let rows: i64 = test().run(
        r#"
            [a b]
            | tui menu [{name: "&File", items: [{name: "&Reload", action: {|| [x y z] }}]}]
            | tui table
            | tui debug --keys "alt+f,r"
            | get rows
        "#,
    )?;
    assert_eq!(rows, 3);
    let action: String = test().run(
        r#"
            [a b]
            | tui menu [{name: "&File", items: [{name: "&Quit", action: {|| {action: quit} }}]}]
            | tui table
            | tui debug --keys "alt+f,q"
            | get action
        "#,
    )?;
    assert_eq!(action, "quit");
    Ok(())
}

#[test]
fn refresh_hook_replaces_rows() -> Result {
    let screen: String = test().run(
        "
            [{name: old}]
            | tui table --columns [name]
            | tui debug { [{name: NEWMARKER}] }
            | get screen
        ",
    )?;
    assert_contains("NEWMARKER", &screen);
    Ok(())
}

#[test]
fn bind_runs_a_hook_with_the_state_record() -> Result {
    let rows: i64 = test().run(
        "[a b] | tui table | tui bind ctrl+r {|| [x y z] } | tui debug --keys ctrl+r | get rows",
    )?;
    assert_eq!(rows, 3);
    let selected: String = test().run(
        "
            [{name: a} {name: b}]
            | tui table
            | tui bind s {|state| {action: submit, selected: $state.selected.name} }
            | tui debug --keys [down s]
            | get selected
        ",
    )?;
    assert_eq!(selected, "b");
    let action: String = test().run(
        "[a b] | tui table | tui bind {modifier: control, keycode: char_q} {|| {action: quit} } | tui debug --keys ctrl+q | get action",
    )?;
    assert_eq!(action, "quit");
    Ok(())
}

#[test]
fn plain_bind_does_not_fire_while_typing() -> Result {
    let selected: String = test().run(
        r#"tui textbox | tui bind s {|| {action: quit} } | tui debug --keys "type:s,enter" | get selected"#,
    )?;
    assert_eq!(selected, "s");
    Ok(())
}

#[test]
fn on_select_hook_runs_when_the_highlight_moves() -> Result {
    let rows: i64 = test().run(
        "[a b] | tui table --on-select {|s| [w x y z] } | tui debug --keys [down] | get rows",
    )?;
    assert_eq!(rows, 4);
    Ok(())
}

#[test]
fn multi_and_index_change_the_selection() -> Result {
    let selected: Vec<String> = test().run(
        "[a b c] | tui table --multi | tui debug --keys [space down space enter] | get selected",
    )?;
    assert_eq!(selected, vec!["a".to_string(), "b".to_string()]);
    let index: i64 =
        test().run("[a b c] | tui table --index | tui debug --keys [down enter] | get selected")?;
    assert_eq!(index, 1);
    Ok(())
}

#[test]
fn select_picks_items() -> Result {
    let selected: String = test()
        .run("tui select [small medium large] | tui debug --keys [down enter] | get selected")?;
    assert_eq!(selected, "medium");
    let names: Vec<String> = test().run(
        "[{name: a, n: 1} {name: b, n: 2}] | tui select --multi --display name | tui debug --keys [space down space enter] | get selected.name",
    )?;
    assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    let screen: String = test().run(
        r#"[{name: a}] | tui select --display {|it| $"ITEM-($it.name)" } | tui debug --size [30 6] | get screen"#,
    )?;
    assert_contains("ITEM-a", &screen);
    Ok(())
}

#[test]
fn buttons_submit_or_run_hooks() -> Result {
    let selected: String = test().run(
        r#"tui label "Delete?" | tui button Yes | tui button No | tui debug --keys [tab enter] | get selected"#,
    )?;
    assert_eq!(selected, "No");
    let action: String =
        test().run("tui button Go {|| {action: quit} } | tui debug --keys enter | get action")?;
    assert_eq!(action, "quit");
    Ok(())
}

#[test]
fn progress_reads_values_and_data() -> Result {
    let screen: String = test()
        .run("tui progress --value 0.4 --label copying | tui debug --size [40 4] | get screen")?;
    assert_contains("copying 40%", &screen);
    let value: f64 =
        test().run("[10 50] | tui progress --total 100 | tui debug | get values.progress-0")?;
    assert_eq!(value, 50.0);
    Ok(())
}

#[test]
fn tui_theme_comes_from_config() -> Result {
    let cols: Vec<String> = test().run("$env.config.tui | columns")?;
    assert!(cols.iter().any(|c| c == "title_bar"), "got {cols:?}");
    let screen: String = test().run(
        r#"$env.config.tui.title_bar = {fg: red}; tui label --titlebar "T" | tui debug | get screen"#,
    )?;
    assert_contains("T", &screen);
    Ok(())
}

#[test]
fn log_shows_the_newest_lines_within_max_lines() -> Result {
    let screen: String =
        test().run("1..30 | tui log --max-lines 5 | tui debug --size [40 8] | get screen")?;
    assert_contains("30", &screen);
    assert!(!screen.contains("20"), "older lines are dropped: {screen}");
    Ok(())
}

#[test]
fn page_digits_beat_capture_keys() -> Result {
    let page: i64 = test().run(
        "[{a: 1}] | tui tab one [(tui table --capture-keys)] | tui tab two [(tui label x)] | tui debug --keys 2 | get page",
    )?;
    assert_eq!(page, 1);
    Ok(())
}

#[test]
fn unknown_from_id_is_an_error() -> Result {
    let err = test()
        .run("[a] | tui table --from nope | tui debug")
        .expect_error()?;
    assert_contains("unknown --from", &format!("{err:?}"));
    Ok(())
}

#[test]
fn from_must_name_a_row_source() -> Result {
    let err = test()
        .run("[a] | tui label hi --id note | tui log --from note | tui debug")
        .expect_error()?;
    assert_contains(
        "--from must name a table, tree, or select",
        &format!("{err:?}"),
    );
    Ok(())
}

#[test]
fn from_cannot_name_the_widget_itself() -> Result {
    let err = test()
        .run(r#"[a b] | tui table --from table-0 {|row| [$"x($row)"] } | tui debug"#)
        .expect_error()?;
    assert_contains("cannot follow itself", &format!("{err:?}"));
    Ok(())
}

#[test]
fn from_ids_in_a_loop_are_an_error() -> Result {
    let err = test()
        .run("tui table --from table-1 {|row| [x] } | tui table --from table-0 {|row| [y] } | tui debug")
        .expect_error()?;
    assert_contains("--from ids form a loop", &format!("{err:?}"));
    Ok(())
}

#[test]
fn closure_follows_a_table_on_another_tab_page() -> Result {
    let screen: String = test().run(
        r#"[a b] | tui tab one [(tui table)] | tui tab two [(tui log {|row| [$"f ($row)"] })] | tui debug --keys ["2"] | get screen"#,
    )?;
    assert_contains("f a", &screen);
    Ok(())
}

#[test]
fn closure_with_no_row_to_follow_is_an_error() -> Result {
    // #19217: the closure used to hide the piped stream and leave the log empty.
    for code in [
        r#"1.. | each {|n| $"tick ($n)"} | tui label --titlebar ticks | tui log { $in } | tui debug"#,
        // A table does not follow its own row.
        "[a] | tui table {|row| [x] } | tui debug",
        "tui label {|row| $row } | tui progress {|row| 1 } | tui debug",
        // Two tables that could only follow each other.
        "tui table {|row| [x] } | tui table {|row| [y] } | tui debug",
    ] {
        let err = test().run(code).expect_error()?;
        assert_contains("nothing for the closure to follow", &format!("{err:?}"));
    }
    Ok(())
}

#[test]
fn own_data_and_a_source_are_incompatible() -> Result {
    for code in [
        "tui log --data [a] {|row| $row }",
        "[a] | tui table | tui table --data [b] --from table-0",
        "[a] | tui table | tui progress --value 0.5 {|row| 1 }",
        "[a] | tui table | tui progress --value 0.5 --from table-0",
    ] {
        let err = test().run(code).expect_error()?;
        assert_contains("not both", &format!("{err:?}"));
    }
    Ok(())
}

#[test]
fn data_piped_into_a_child_skips_widgets_that_follow_a_row() -> Result {
    let screen: String = test().run(
        r#"tui split [([a b] | tui table | tui log {|row| [$"follows ($row)"] })] | tui debug --size [40 6] | get screen"#,
    )?;
    assert_contains("follows a", &screen);
    let err = test()
        .run("[a] | tui table | tui split [([b] | tui log --from table-0)] | tui debug")
        .expect_error()?;
    assert_contains("piped data has no widget", &format!("{err:?}"));
    Ok(())
}

#[test]
fn root_search_sits_above_the_content() -> Result {
    let ys: Vec<i64> = test()
        .run("[a b] | tui search | tui table | tui debug --size [40 12] | get widgets.rect.y")?;
    assert_eq!(ys, vec![0, 3]);
    Ok(())
}

#[test]
fn ls_rows_expand_in_a_tree() -> Result {
    let screen: String = test().run(
        r#"[{name: "a.txt", type: file, size: 1}] | tui tree | tui debug --keys right --size [40 6] | get screen"#,
    )?;
    assert_contains("type: file", &screen);
    Ok(())
}

#[test]
fn buttons_flow_onto_one_row_and_split_vertical_stacks_them() -> Result {
    let screen: String = test().run(
        r#"tui label "Delete everything?" | tui button Yes | tui button No | tui debug --size [40 4] | get screen"#,
    )?;
    let row = screen.lines().nth(1).unwrap_or_default();
    assert_contains("[ Yes ]", row);
    assert_contains("[ No ]", row);
    let heights: Vec<i64> = test().run(
        "tui split --vertical [(tui button Yes) (tui button No)] | tui table --data [a] | tui debug | get widgets.0.children.rect.y",
    )?;
    assert_eq!(heights, vec![0, 1]);
    let selected: String = test()
        .run("tui button Yes | tui button No | tui debug --keys [right enter] | get selected")?;
    assert_eq!(selected, "No");
    Ok(())
}

#[test]
fn focus_flag_sets_the_initial_focus() -> Result {
    let name: String = test().run(
        r#"[{name: alpha}, {name: beta}] | tui search --focus --columns [name] | tui table | tui debug --keys "type:be,enter" | get selected.name"#,
    )?;
    assert_eq!(name, "beta");
    let focused: String =
        test().run("tui textbox --id a | tui textbox --id b --focus | tui debug | get focused")?;
    assert_eq!(focused, "b");
    Ok(())
}

#[test]
fn closures_see_their_pane_size() -> Result {
    let text: String = test().run(
        "[{name: a}] | tui split --sizes [40 1fr] [(tui table) (tui preview {|r| $\"($env.TUI_WIDTH)x($env.TUI_HEIGHT)\" })] | tui debug --size [80 24] | get values.preview-0.text",
    )?;
    // 80 wide: 40 for the table, 1 divider, 39 for the preview, minus its border.
    assert_eq!(text, "37x22");
    let rows: i64 = test().run(
        "[{name: a}] | tui split [(tui table --id src) (tui table --from src {|r| 1..$env.TUI_HEIGHT })] | tui debug --size [80 24] | get widgets.0.children.1.rows",
    )?;
    assert_eq!(rows, 22);
    Ok(())
}

/// Writes `img.png`: 8 by 4 pixels, red on the left and blue on the right.
const SAVE_PNG: &str = "0x[89504e470d0a1a0a0000000d4948445200000008000000040806000000b3cd7ef00000004a494441547801edc003a0245996c6f1ff77ee8dc8cca7724b63ae6ddbb66ddbb66ddbb66d698c9e964aaf9e323322eef976b76a7aa6873b6bd5af1acc0308f300042f1cc10b47f0c2f18f772e0309d0272ed70000000049454e44ae426082] | save -f img.png";

#[test]
fn preview_draws_an_image_file_headless_as_half_blocks() -> Result {
    Playground::setup("tui_preview_image", |dirs, _| {
        let image: String = test().cwd(dirs.test()).run(format!(
            r#"
                {SAVE_PNG}
                ls
                | tui split [(tui table --columns [name]) (tui preview)]
                | tui debug --size [60 12]
                | get values.preview-0.image
                | $"($in.width)x($in.height) ($in.format) ($in.protocol) ($in.columns)x($in.rows)"
            "#
        ))?;
        // Headless runs use half blocks at a 10x20 cell, so 8x4 pixels is one cell.
        assert_eq!(image, "8x4 png halfblocks 1x1");

        let screen: String = test().cwd(dirs.test()).run(format!(
            "
                {SAVE_PNG}
                ls
                | tui split [(tui table --columns [name]) (tui preview)]
                | tui debug --size [60 12]
                | get screen
            "
        ))?;
        assert_contains("img.png", &screen);
        assert_contains("▀", &screen);
        Ok(())
    })
}

#[test]
fn image_widget_scales_within_its_caps_and_keeps_a_manual_id() -> Result {
    Playground::setup("tui_image_widget", |dirs, _| {
        // The pane is 58 cells wide inside its border; `--image-width 10`
        // caps the box at 10x10 cells, which is 100x200 pixels. The 2:1
        // image scales to 100x50 pixels: 10 columns and 3 rows.
        let image: String = test().cwd(dirs.test()).run(format!(
            r#"
                {SAVE_PNG}
                tui image img.png --image-fit scale --image-width 10 --image-id 7
                | tui debug --size [60 12]
                | get values.image-0.image
                | $"($in.columns)x($in.rows) ($in.id)"
            "#
        ))?;
        assert_eq!(image, "10x3 7");

        // Without `--image-id` the run allocates one.
        let auto_id: i64 = test().cwd(dirs.test()).run(format!(
            "{SAVE_PNG}; tui image img.png | tui debug | get values.image-0.image.id"
        ))?;
        assert!(auto_id > 0, "{auto_id}");

        // Bytes work as the source too, and `describe` shows the options.
        let from_bytes: String = test().cwd(dirs.test()).run(format!(
            r#"{SAVE_PNG}; tui image (open --raw img.png) --image-fit crop | tui debug | $"($in.values.image-0.image.format) ($in.widgets.0.image_fit)""#
        ))?;
        assert_eq!(from_bytes, "png crop");
        Ok(())
    })
}

#[test]
fn image_that_cannot_be_decoded_or_drawn_is_described_instead() -> Result {
    Playground::setup("tui_image_notes", |dirs, _| {
        let note: String = test().cwd(dirs.test()).run(
            "0x[89504e470d0a1a0a00] | save -f bad.png; tui image bad.png | tui debug | get values.image-0.text",
        )?;
        assert_contains("cannot decode", &note);

        let described: String = test().cwd(dirs.test()).run(format!(
            r#"
                {SAVE_PNG}
                $env.config.tui.image_protocol = "none"
                tui image img.png | tui debug | $"($in.values.image-0.text) ($in.values.image-0.image | describe)"
            "#
        ))?;
        assert_eq!(described, "(image 8x4 png, 131 bytes) nothing");
        Ok(())
    })
}

#[test]
fn scoped_binds_pass_the_key_on_when_the_hook_returns_nothing() -> Result {
    Playground::setup("tui_bind_when", |dirs, _| {
        // PgDn is bound only while the preview is focused. The hook returns
        // nothing for `lines.txt`, so the pane's own paging still happens;
        // it submits for anything else.
        let out: String = test().cwd(dirs.test()).run(
            r#"
                1..60 | each { $"line ($in)" } | str join "
" | save -f lines.txt
                "other" | save -f other.txt
                let app = {|| ls | tui split [(tui table --id files --columns [name]) (tui preview --id view)]
                    | tui bind --when view pagedown {|s| if $s.values.files.row.name == "lines.txt" { null } else { {action: submit, selected: claimed} } } }
                let paged = do $app | tui debug --size [60 14] --keys [tab pagedown] | $"($in.values.view.scroll) ($in.action)"
                let claimed = do $app | tui debug --size [60 14] --keys [down tab pagedown] | get selected
                let unfocused = do $app | tui debug --size [60 14] --keys [down pagedown] | $"($in.action) ($in.selected.name)"
                $"($paged) | ($claimed) | ($unfocused)"
            "#,
        )?;
        // 12 inner rows paged once; claimed when focused on other.txt; and
        // with the table focused the bind is dormant, so PgDn moves the table.
        assert_eq!(out, "12 render | claimed | render other.txt");
        Ok(())
    })
}

#[test]
fn context_is_shared_by_hooks_and_split_per_view() -> Result {
    // `tui context` merges into the `tui` value; hooks read it as
    // `$state.context` and write it with an id-less `context` action.
    let seeded: String = test().run(
        "tui table | tui context {dir: /a, n: 0} | tui context {n: 1} | get context | to nuon",
    )?;
    assert_eq!(seeded, "{dir: /a, n: 1}");
    let counted: String = test().run(
        "[1 2] | tui table | tui context {n: 0}
            | tui bind x {|s| {action: context, value: {n: ($s.context.n + 1)}} }
            | tui debug --keys [x x x] | get context.n | into string",
    )?;
    assert_eq!(counted, "3");

    Playground::setup("tui_context_views", |dirs, _| {
        // A preview's closure sees the shared keys under its own `views`
        // part, which `--on-change` starts for the new file (`values.view.row`).
        // It runs again for a change to either, not for a write to another
        // widget's part; an unknown id is a hook error.
        let out: String = test().cwd(dirs.test()).run(
            r#"
                "x" | save -f a.txt
                ls a.txt
                | tui split [(tui table --id files --columns [name]) (tui preview --id view --on-change {|s| {page: 1, file: $s.values.view.row.name} } {|row, ctx| "." | save --append runs.txt; $"w($ctx.width) p($ctx.page)" })]
                | tui label --status "demo"
                | tui context {width: 10}
                | tui bind w {|| {action: context, value: {width: 20}} }
                | tui bind p {|| {action: context, id: view, value: {page: 2}} }
                | tui bind o {|| {action: context, id: files, value: {x: 1}} }
                | tui bind n {|| {action: context, id: nope, value: {x: 1}} }
                | tui debug --size [60 8] --keys [o w p n]
                | $"($in.values.view.text) | ($in.context | to nuon) | ($in.values.view.context | to nuon) | (open runs.txt) | ($in.screen | lines | last)"
            "#,
        )?;
        assert_contains(
            "w20 p2 | {width: 20, views: {view: {page: 2, file: \"a.txt\"}, files: {x: 1}}} | {width: 20, page: 2, file: \"a.txt\"} | ... |",
            &out,
        );
        assert_contains("`nope` is not a widget", &out);
        Ok(())
    })
}

#[test]
fn hook_actions_change_one_widget() -> Result {
    Playground::setup("tui_hook_actions", |dirs, _| {
        // `context` merges into `$ctx` and re-runs the closure; `--on-change`
        // seeds it per file, and its `hint` is the status hint.
        let paged: String = test().cwd(dirs.test()).run(
            r#"
                "x" | save -f a.pdf
                ls a.pdf
                | tui split [(tui table --columns [name]) (tui preview --id view --on-change {|| {page: 1, hint: "pgdn: page"} } {|row, ctx| $"($row.name) page ($ctx.page)" })]
                | tui label --status "demo"
                | tui bind --when view pagedown {|s| {action: context, id: view, value: {page: ($s.values.view.context.page + 1)}} }
                | tui debug --size [60 10] --keys [tab pagedown pagedown]
                | $"($in.values.view.text) | ($in.values.view.context | to nuon) | ($in.screen | lines | last)"
            "#,
        )?;
        assert_contains("a.pdf page 3 | {page: 3, hint: \"pgdn: page\"} |", &paged);
        assert_contains("pgdn: page", &paged);

        // `scroll` moves the text, negative from the end; `resize` and
        // `focus` as a list apply in order.
        let moved: String = test().cwd(dirs.test()).run(
            r#"
                1..60 | each { $"line ($in)" } | str join "
" | save -f lines.txt
                ls lines.txt
                | tui split --id layout --sizes [50% 1fr] [(tui table --id files --columns [name]) (tui preview --id view)]
                | tui bind g {|| {action: scroll, id: view, line: 40} }
                | tui bind shift+g {|| {action: scroll, id: view, line: -1} }
                | tui bind f {|| [{action: resize, id: layout, sizes: [0 1fr]} {action: focus, id: view}] }
                | tui debug --size [60 14] --keys [g]
                | $"($in.values.view.scroll)"
            "#,
        )?;
        assert_eq!(moved, "40");
        let end: String = test().cwd(dirs.test()).run(
            r#"
                ls lines.txt
                | tui split --id layout --sizes [50% 1fr] [(tui table --id files --columns [name]) (tui preview --id view)]
                | tui bind shift+g {|| {action: scroll, id: view, line: -1} }
                | tui bind f {|| [{action: resize, id: layout, sizes: [0 1fr]} {action: focus, id: view}] }
                | tui debug --size [60 14] --keys [shift+g f]
                | $"($in.values.view.scroll) ($in.values.layout | to nuon) ($in.focused)"
            "#,
        )?;
        // 60 lines in 12 inner rows: the last page starts at 48.
        assert_eq!(end, "48 [0, \"1fr\"] view");
        Ok(())
    })
}

#[test]
fn preview_threads_gives_the_same_result_from_a_thread() -> Result {
    Playground::setup("tui_preview_threads", |dirs, _| {
        // Headless runs wait for every job, so the thread is invisible
        // except in the result: a file read, a transform, and a closure
        // that returns image bytes all match their synchronous twins.
        let both: String = test().cwd(dirs.test()).run(format!(
            r#"
                {SAVE_PNG}
                "hello" | save -f notes.txt
                let app = {{|flags| ls notes.txt | tui split [(tui table --columns [name]) (tui preview --id v ...$flags {{ str upcase }})] | tui debug --size [60 8] | get values.v.text }}
                let img = {{|flags| ls img.png | tui split [(tui table --columns [name]) (tui preview --id v ...$flags {{|row| open --raw $row.name }})] | tui debug --size [60 8] | get values.v.image | $"($in.width)x($in.height) ($in.format)" }}
                $"(do $app {{}}) (do $app {{threads: true}}) (do $img {{}}) (do $img {{threads: true}})"
            "#
        ))?;
        assert_eq!(both, "HELLO HELLO 8x4 png 8x4 png");
        Ok(())
    })
}

#[test]
fn focused_preview_scrolls_text_and_zooms_images() -> Result {
    Playground::setup("tui_preview_keys", |dirs, _| {
        // 60 lines in a 10-row pane: Tab to the preview, PageDown once.
        let screen: String = test().cwd(dirs.test()).run(
            r#"
                1..60 | each { $"line ($in)" } | str join "\n" | save -f lines.txt
                ls lines.txt
                | tui split [(tui table --columns [name]) (tui preview)]
                | tui label --status "demo"
                | tui debug --size [60 14] --keys [tab pagedown]
                | $"($in.focused)\n($in.screen)"
            "#,
        )?;
        assert_contains("preview-0\n", &screen);
        assert_contains("pgdn:scroll", &screen);
        assert_contains("line 15", &screen);
        assert!(!screen.contains("line 1 "), "{screen}");

        // Up at the top stays at the top rather than counting from the end.
        let top: i64 = test().cwd(dirs.test()).run(
            "
                ls lines.txt
                | tui split [(tui table --columns [name]) (tui preview --id view)]
                | tui debug --size [60 14] --keys [tab down up up pageup]
                | get values.view.scroll
            ",
        )?;
        assert_eq!(top, 0);

        // `=` zooms in by 1.25 twice; `0` goes back to the first view.
        let zoomed: String = test().cwd(dirs.test()).run(format!(
            r#"
                {SAVE_PNG}
                tui image img.png --focus
                | tui debug --size [60 12] --keys [= =]
                | get values.image-0.image
                | $"($in.columns)x($in.rows) ($in.zoom)"
            "#
        ))?;
        assert_eq!(zoomed, "2x1 1.5625");
        let reset: f64 = test().cwd(dirs.test()).run(format!(
            "{SAVE_PNG}; tui image img.png --focus | tui debug --keys [= = 0] | get values.image-0.image.zoom"
        ))?;
        assert_eq!(reset, 1.0);
        // `i` switches the resampling; `0` resets the zoom but keeps it.
        let filters: String = test().cwd(dirs.test()).run(format!(
            r#"{SAVE_PNG}; [[] [i] [i i] [i "0"]] | each {{|keys| tui image img.png --focus | tui debug --keys $keys | get values.image-0.image.filter }} | str join ','"#
        ))?;
        assert_eq!(filters, "linear,nearest,linear,nearest");
        Ok(())
    })
}

#[test]
fn bind_sequences_wait_for_their_second_key() -> Result {
    // `m` alone shows the panel; `m s` runs the hook; a key that continues
    // nothing ends the sequence and goes nowhere itself.
    let run = |keys: &str| -> Result<String> {
        test().run(format!(
            r#"
                [{{a: 1, b: 2}}]
                | tui table --id t --columns [a]
                | tui label --status "demo"
                | tui bind "m b" --label "show b" {{|| {{action: columns, id: t, columns: [b]}} }}
                | tui bind "m a" --label "show a" {{|| {{action: columns, id: t, columns: [a]}} }}
                | tui bind x {{|| {{action: quit}} }}
                | tui debug --size [40 10] --keys [{keys}]
                | $"($in.action) ($in.widgets.0.resolved_columns | str join ',')\n($in.screen)"
            "#
        ))
    };
    let pending = run("m")?;
    assert_contains("a → show a", &pending);
    assert_contains("b → show b", &pending);
    assert_contains("render a\n", &pending);
    let done = run("m b")?;
    assert_contains("render b\n", &done);
    assert!(!done.contains("show b"), "{done}");
    // `x` after `m` is swallowed with the sequence; the next `x` quits.
    assert_contains("render a\n", &run("m x")?);
    assert_contains("quit a\n", &run("m x x")?);
    Ok(())
}

#[test]
fn a_suspended_hook_gets_its_area_and_its_result_is_applied() -> Result {
    // Headless there is no terminal to lend, but the hook still runs with
    // the area of the widget it named, and its result lands as usual.
    let out: String = test().run(
        r#"
            [{a: 1}]
            | tui table --id t
            | tui bind z {|| {action: suspend, area: t, hook: {|s| [{a: ($"($s.area | reject screen | values | str join x) of ($s.area.screen.width)")}] }} }
            | tui bind q {|| {action: suspend, hook: {|s| {action: submit, selected: ($s | get -o area | describe)} }} }
            | tui bind e {|| {action: suspend, area: nope, hook: {|| [] }} }
            | tui debug --size [30 6] --keys [z] | get values.t.row.a
        "#,
    )?;
    assert_eq!(out, "0x0x30x6 of 30");
    let plain: String = test().run(
        "
            [{a: 1}] | tui table --id t
            | tui bind q {|| {action: suspend, hook: {|s| {action: submit, selected: ($s | get -o area | describe)} }} }
            | tui debug --keys [q] | get selected
        ",
    )?;
    assert_eq!(plain, "nothing");
    let bad: String = test().run(
        r#"
            [{a: 1}] | tui table --id t | tui label --status "demo"
            | tui bind e {|| {action: suspend, area: nope, hook: {|| [] }} }
            | tui debug --keys [e] | get screen
        "#,
    )?;
    assert_contains("`nope` is not a widget on screen", &bad);
    Ok(())
}

#[test]
fn a_hook_can_replace_the_rows_and_pick_the_highlight_together() -> Result {
    let picked: i64 = test().run(
        "
            [{a: 1}] | tui table --id t
            | tui bind b {|| [{action: data, value: [{a: 1} {a: 2} {a: 3}]} {action: select, id: t, index: 2}] }
            | tui bind c {|| {action: select, id: t, index: 99} }
            | tui debug --keys [b] | get values.t.row.a
        ",
    )?;
    assert_eq!(picked, 3);
    // Past the end lands on the last row.
    let clamped: i64 = test().run(
        "
            [{a: 1} {a: 2}] | tui table --id t
            | tui bind c {|| {action: select, id: t, index: 99} }
            | tui debug --keys [c] | get values.t.row.a
        ",
    )?;
    assert_eq!(clamped, 2);
    Ok(())
}

#[test]
fn icons_go_before_path_cells_and_color_names_a_column_of_bare_names() -> Result {
    Playground::setup("tui_table_icons", |dirs, _| {
        // `--icons` draws devicons' icon before a path cell, the one
        // `table --icons` draws: a file's by its name, a directory's for
        // one on disk. With `--color`, a column of bare names gets the
        // same, looked up through the row's full `name`.
        let rust = devicons::icon_for_file(std::path::Path::new("main.rs"), &None).icon;
        let folder = devicons::icon_for_file(&dirs.test().join("sub"), &None).icon;
        assert_ne!(rust, folder);
        let screen: String = test().cwd(dirs.test()).run(
            "
                mkdir sub; 'fn main() {}' | save -f main.rs
                ls
                | insert file { $in.name | path basename }
                | tui table --columns [file] --icons --color file
                | tui debug --size [30 6]
                | get screen
            ",
        )?;
        assert_contains(format!("{rust} main.rs"), &screen);
        assert_contains(format!("{folder} sub"), &screen);

        // Without `--icons` the names are bare, and a column that is not a
        // path keeps its text as it is.
        let plain: String = test().cwd(dirs.test()).run(
            "
                ls | insert file { $in.name | path basename }
                | tui table --columns [file]
                | tui debug --size [30 6] | get screen
            ",
        )?;
        assert!(!plain.contains(rust), "{plain}");
        assert_contains("main.rs", &plain);
        Ok(())
    })
}

#[cfg(unix)]
#[test]
fn file_styles_rules_look_at_the_file_on_disk_and_set_its_icon() -> Result {
    Playground::setup("tui_file_styles", |dirs, _| {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = dirs.test();
        std::fs::create_dir(root.join("sub"))?;
        for name in ["a.nu", "run.sh", "b.txt"] {
            std::fs::write(root.join(name), "x")?;
        }
        std::fs::set_permissions(root.join("run.sh"), PermissionsExt::from_mode(0o755))?;
        std::fs::set_permissions(root.join("a.nu"), PermissionsExt::from_mode(0o644))?;
        std::fs::set_permissions(root.join("b.txt"), PermissionsExt::from_mode(0o644))?;
        // A link to a directory is a symlink, not a directory.
        symlink("sub", root.join("link"))?;
        symlink("missing", root.join("gone"))?;
        let screen: String = test().cwd(root).run(
            r#"
                ls
                | tui table --columns [name] --icons
                | tui theme {file_styles: [
                    {kind: orphan, icon: "O"}
                    {kind: symlink, icon: "L"}
                    {kind: dir, icon: "D"}
                    {mode: "??x*", icon: "X"}
                    {ext: NU, icon: "N"}
                ]}
                | tui debug --size [30 10]
                | get screen
            "#,
        )?;
        for cell in ["O gone", "L link", "D sub", "X run.sh", "N a.nu"] {
            assert_contains(cell, &screen);
        }
        // No rule matches: devicons' own glyph.
        let text = devicons::icon_for_file(std::path::Path::new("b.txt"), &None).icon;
        assert_contains(format!("{text} b.txt"), &screen);
        Ok(())
    })
}

#[test]
fn delete_image_writes_to_the_terminal_and_returns_nothing() -> Result {
    // The sequence goes straight to stdout (`image::tests` checks its
    // bytes); the pipeline itself carries nothing.
    let out: String = test().run("tui delete-image 7 | describe")?;
    assert_eq!(out, "nothing");
    Ok(())
}
