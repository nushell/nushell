use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;

#[test]
fn parent_command_lists_subcommands() -> Result {
    let help: String = test().run("tui")?;
    assert_contains("tui run", &help);
    assert_contains("tui table", &help);
    assert_contains("tui search", &help);
    Ok(())
}

#[test]
fn headless_renders_title_and_status() -> Result {
    let screen: String =
        test().run(r#"tui title "Demo" | tui status "ready" | tui run --headless"#)?;
    assert_contains("Demo", &screen);
    assert_contains("ready", &screen);
    Ok(())
}

#[test]
fn table_keys_submit_selected_row() -> Result {
    let code = r#"
        [{name: alpha, n: 1}, {name: beta, n: 2}, {name: gamma, n: 3}]
        | tui table --columns [name n]
        | tui run --keys down,enter
    "#;
    let action: String = test().run(&format!("{code} | get action"))?;
    assert_eq!(action, "submit");
    let name: String = test().run(&format!("{code} | get selected.name"))?;
    assert_eq!(name, "beta");
    Ok(())
}

#[test]
fn search_filters_table_before_submit() -> Result {
    let code = r#"
        [{name: alpha}, {name: beta}, {name: gamma}]
        | tui search --placeholder "filter" --bind /
        | tui table --columns [name]
        | tui run --keys "/,type:ga,tab,enter"
    "#;
    let name: String = test().run(&format!("{code} | get selected.name"))?;
    assert_eq!(name, "gamma");
    Ok(())
}

#[test]
fn textbox_typing_does_not_quit_on_q() -> Result {
    let code = r#"
        tui textbox --placeholder "name"
        | tui run --keys "type:q,enter"
    "#;
    let action: String = test().run(&format!("{code} | get action"))?;
    assert_eq!(action, "submit");
    let selected: String = test().run(&format!("{code} | get selected"))?;
    assert_eq!(selected, "q");
    Ok(())
}

#[test]
fn q_quits_table_without_submit() -> Result {
    let action: String =
        test().run(r#"[{name: a}] | tui table | tui run --keys q | get action"#)?;
    assert_eq!(action, "quit");
    Ok(())
}

#[test]
fn pipeline_app_is_custom_tui_value() -> Result {
    let desc: String = test().run(r#"tui title "x" | describe"#)?;
    assert_contains("tui", &desc);
    Ok(())
}

#[test]
fn keybindings_headless_shows_name() -> Result {
    let screen: String = test().run(
        r#"
            [{name: "history", modifier: "control", keycode: "char_r", mode: "emacs", event: {send: "OpenHistory"}}]
            | tui keybindings
            | tui run --headless
        "#,
    )?;
    assert_contains("history", &screen);
    Ok(())
}

#[test]
fn tab_pages_switch_with_bracket() -> Result {
    let screen: String = test().run(
        r#"
            tui body "one"
            | tui label "PAGE-ONE"
            | tui body "two"
            | tui label "PAGE-TWO"
            | tui run --keys "]" --headless
            | get screen
        "#,
    )?;
    assert_contains("PAGE-TWO", &screen);
    Ok(())
}

#[test]
fn keybindings_chord_selects_matching_row() -> Result {
    let name: String = test().run(
        r#"
            [
                {name: "history", modifier: "control", keycode: "char_r", mode: "emacs", event: {send: "OpenHistory"}}
                {name: "clear", modifier: "control", keycode: "char_l", mode: "emacs", event: {send: "Clear"}}
            ]
            | tui keybindings
            | tui run --keys "ctrl+r,enter"
            | get selected.name
        "#,
    )?;
    assert_eq!(name, "history");
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
            r#"
                ls
                | sort-by name
                | tui splitter --direction horizontal --ratio 50
                | tui table --columns [name type]
                | tui preview
                | tui run --headless --width 80 --height 16
            "#,
        )?;
        assert_contains("ONE-FILE-BODY", &screen);

        let screen: String = test().cwd(dirs.test()).run(
            r#"
                ls
                | sort-by name
                | tui splitter --direction horizontal --ratio 50
                | tui table --columns [name type]
                | tui preview
                | tui run --keys down --headless --width 80 --height 16
                | get screen
            "#,
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
                | tui preview { $in | str upcase }
                | tui table
                | tui run --headless --width 80 --height 16
            "#,
        )?;
        assert_contains("HELLO-PREVIEW", &screen);
        Ok(())
    })
}

#[test]
fn builders_do_not_collect_an_infinite_stream() -> Result {
    test()
        .run("1.. | tui title 'x' | tui list | first")
        .expect_value_eq(1)
}

#[test]
fn streamed_list_reaches_the_tui() -> Result {
    let screen: String = test().run(
        r#"
            1..5
            | each {|n| sleep 1ms; $n}
            | tui title "stream"
            | tui list
            | tui run --headless --width 40 --height 12
        "#,
    )?;
    assert_contains("5", &screen);
    assert_contains("stream", &screen);
    Ok(())
}

#[test]
fn dialog_headless_draws_close_control() -> Result {
    let screen: String = test().run(
        r#"tui title "popup" | tui status "hi" | tui run --dialog --headless --width 80 --height 24"#,
    )?;
    assert_contains("popup", &screen);
    assert_contains("x", &screen);
    Ok(())
}

#[test]
fn menu_items_reject_non_strings() -> Result {
    test()
        .run(r#"tui menu --items {a: 1} | tui run --headless"#)
        .expect_parse_error()?;
    Ok(())
}

#[test]
fn widgets_field_is_inspectable() -> Result {
    let types: Vec<String> =
        test().run(r#"tui title "App" | tui status "ok" | get widgets.type"#)?;
    assert_eq!(types, vec!["title".to_string(), "status".to_string()]);
    Ok(())
}

#[test]
fn parent_help_lists_new_widgets() -> Result {
    let help: String = test().run("tui")?;
    assert_contains("tui tree", &help);
    assert_contains("tui tab", &help);
    assert_contains("tui tabs", &help);
    Ok(())
}

#[test]
fn named_tabs_switch_with_digit() -> Result {
    let screen: String = test().run(
        r#"
            tui tab "one"
            | tui label "PAGE-ONE"
            | tui tab "two"
            | tui label "PAGE-TWO"
            | tui run --keys "2" --headless
            | get screen
        "#,
    )?;
    assert_contains("PAGE-TWO", &screen);
    Ok(())
}

#[test]
fn tree_expands_nested_record() -> Result {
    let screen: String = test().run(
        r#"
            {a: {b: 1, c: 2}, d: [3, 4]}
            | tui tree
            | tui run --keys "right,down,right,down,right" --headless --width 40 --height 16
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
fn placement_right_of_renders_both() -> Result {
    let screen: String = test().run(
        r#"
            tui label "LEFT" --id left
            | tui label "RIGHT" --id right --right-of left
            | tui run --headless --width 40 --height 8
        "#,
    )?;
    assert_contains("LEFT", &screen);
    assert_contains("RIGHT", &screen);
    Ok(())
}

#[test]
fn refresh_closure_replaces_rows() -> Result {
    let screen: String = test().run(
        r#"
            [{name: old}]
            | tui table --columns [name]
            | tui run --headless --refresh 1sec { [{name: NEWMARKER}] }
        "#,
    )?;
    assert_contains("NEWMARKER", &screen);
    Ok(())
}
