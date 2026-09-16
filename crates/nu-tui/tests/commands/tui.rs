use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn parent_command_lists_subcommands() -> Result {
    let help: String = test().run("tui")?;
    assert_contains("tui run", &help);
    assert_contains("tui debug", &help);
    assert_contains("tui table", &help);
    assert_contains("tui search", &help);
    assert_contains("tui split", &help);
    assert_contains("tui tab", &help);
    assert_contains("tui tree", &help);
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
fn tab_pages_switch_with_bracket() -> Result {
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
    Ok(())
}

#[test]
fn named_tabs_switch_with_digit() -> Result {
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
    let screen: String =
        test().run(r#"tui tab "only" [(tui label "x")] | tui debug | get screen"#)?;
    assert_contains(" only", &screen);
    Ok(())
}

#[test]
fn nested_tab_is_a_group_box() -> Result {
    let screen: String = test().run(
        r#"
            tui split [(tui tab "left" [(tui label "A")]) (tui label "B")]
            | tui debug --size [40 8]
            | get screen
        "#,
    )?;
    assert_contains(" left ", &screen);
    assert_contains("┌", &screen);
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
    let value_hits = screen.matches("value").count();
    assert!(
        value_hits == 0,
        "tree should not invent nested 'value' nodes, got {screen:?}"
    );
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
    assert_contains("LEFT", &screen);
    assert_contains("RIGHT", &screen);
    let line = screen
        .lines()
        .find(|l| l.contains("LEFT"))
        .expect("a line with LEFT");
    assert_contains("RIGHT", line);
    Ok(())
}

#[test]
fn split_rejects_children_with_data() -> Result {
    let err = test()
        .run("tui split [(ls | tui table)] | tui debug")
        .expect_error()?;
    assert_contains("tui value", &format!("{err:?}"));
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
    assert_eq!(
        source, "table-1",
        "--from inside the child follows the renumbered id"
    );
    Ok(())
}

#[test]
fn split_rejects_duplicate_explicit_ids() -> Result {
    let err = test()
        .run("tui split [(tui table --id x) (tui table --id x)] | tui debug")
        .expect_error()?;
    assert_contains("duplicate", &format!("{err:?}"));
    Ok(())
}

#[test]
fn split_rejects_nested_chrome() -> Result {
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
    let screen: String = test().run("1..3 | tui table | tui debug --size [30 8] | get screen")?;
    assert_contains("item", &screen);
    assert_contains("3", &screen);
    Ok(())
}

#[test]
fn enter_in_search_box_submits() -> Result {
    let name: String = test().run(
        r#"
            [{name: alpha}, {name: beta}, {name: gamma}]
            | tui search --bind /
            | tui table
            | tui debug --keys "/,type:ga,enter"
            | get selected.name
        "#,
    )?;
    assert_eq!(name, "gamma");
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
    let screen: String = test().run(
        r#"
            [a b]
            | tui menu [{name: "&File", items: ["&Open" "&Quit"]}]
            | tui table
            | tui debug --keys "alt+f" --size [40 8]
            | get screen
        "#,
    )?;
    assert_contains("Open", &screen);
    assert_contains("Quit", &screen);
    Ok(())
}

#[test]
fn menu_action_replaces_rows() -> Result {
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
    Ok(())
}

#[test]
fn refresh_closure_replaces_rows() -> Result {
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
