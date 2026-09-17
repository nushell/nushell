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
fn headless_renders_title_and_status() -> Result {
    let screen: String = test()
        .run(r#"tui label --title "Demo" | tui label --status "ready" | tui debug | get screen"#)?;
    assert_contains("Demo", &screen);
    assert_contains("ready", &screen);
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
    let desc: String = test().run(r#"tui label --title "x" | describe"#)?;
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
fn builders_do_not_collect_an_infinite_stream() -> Result {
    test()
        .run("1.. | tui label --title 'x' | tui table | first")
        .expect_value_eq(1)
}

#[test]
fn streamed_rows_reach_the_tui() -> Result {
    let screen: String = test().run(
        r#"
            1..5
            | each {|n| sleep 1ms; $n}
            | tui label --title "stream"
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
        r#"tui label --title "popup" | tui label --status "hi" | tui debug --dialog --size [80 24] | get screen"#,
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
        test().run(r#"tui label --title "App" | tui label --status "ok" | get widgets.slot"#)?;
    assert_eq!(slots, vec!["title".to_string(), "status".to_string()]);
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
        .run(r#"tui split [(tui label --title "x")] | tui debug"#)
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
        r#"$env.config.tui.title_bar = {fg: red}; tui label --title "T" | tui debug | get screen"#,
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
