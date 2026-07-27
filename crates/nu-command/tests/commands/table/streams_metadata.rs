use indoc::indoc;
use nu_test_support::prelude::*;

#[test]
fn configure_batch_duration() -> Result {
    let expected_default = indoc! {"
        ╭───┬─────────────╮
        │ 0 │ after 1 sec │
        ╰───┴─────────────╯
        ╭───┬─────────────╮
        │ 1 │ after 2 sec │
        ╰───┴─────────────╯
        ╭───┬─────────────╮
        │ 2 │ after 3 sec │
        ╰───┴─────────────╯
        ╭───┬─────────────╮
        │ 3 │ after 4 sec │
        ╰───┴─────────────╯
        ╭───┬─────────────╮
        │ 4 │ after 5 sec │
        ╰───┴─────────────╯
        ╭───┬─────────────╮
        │ 5 │ after 6 sec │
        ╰───┴─────────────╯
    "};
    let actual: String = test()
        .run(r#"1..=6 | each {|i| sleep 1sec; $i | into string | $"after ($in) sec"} | table"#)?;
    assert_eq!(actual, expected_default);
    let expected_two_sec = indoc! {"
        ╭───┬─────────────╮
        │ 0 │ after 1 sec │
        │ 1 │ after 2 sec │
        ╰───┴─────────────╯
        ╭───┬─────────────╮
        │ 2 │ after 3 sec │
        │ 3 │ after 4 sec │
        ╰───┴─────────────╯
        ╭───┬─────────────╮
        │ 4 │ after 5 sec │
        │ 5 │ after 6 sec │
        ╰───┴─────────────╯
    "};
    let actual: String = test().run(
        r#"
        $env.config.table.batch_duration = 2sec
        1..=6 | each {|i| sleep 1sec; $i | into string | $"after ($in) sec"}
        | table
    "#,
    )?;
    assert_eq!(actual, expected_two_sec);
    Ok(())
}

#[test]
fn configure_stream_size() -> Result {
    let expected_default = indoc! {"
        ╭───┬────────╮
        │ 0 │ item 1 │
        │ 1 │ item 2 │
        │ 2 │ item 3 │
        │ 3 │ item 4 │
        ╰───┴────────╯
    "};
    let actual: String = test().run(r#"1..4 | each {"item " + ($in | into string)} | table"#)?;
    assert_eq!(actual, expected_default);
    let expected_size_2 = indoc! {"
        ╭───┬────────╮
        │ 0 │ item 1 │
        │ 1 │ item 2 │
        ╰───┴────────╯
        ╭───┬────────╮
        │ 2 │ item 3 │
        │ 3 │ item 4 │
        ╰───┴────────╯
    "};
    let actual: String = test().run(
        r#"
        $env.config.table.stream_page_size = 2
        1..4 | each {"item " + ($in | into string)}
        | table
    "#,
    )?;
    assert_eq!(actual, expected_size_2);
    Ok(())
}

// Regression test for https://github.com/nushell/nushell/issues/17032
// `table -i false` should not panic when there's an `index` column
#[test]
fn metadata_path_columns_single() -> Result {
    let expected = indoc! {"
        \u{1b}[39m╭───┬──────╮\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m#\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[1;32mname\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m├───┼──────┤\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m0\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[39m\u{1b}[38;5;81msrc\u{1b}[0m\u{1b}[0m  \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m╰───┴──────╯\u{1b}[0m
    "};
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.use_ansi_coloring = true
                $env.config.shell_integration.osc8 = false
                $data | metadata set --path-columns [name] | table
            ",
            test_value!([
                { name: "src" },
            ]),
        )
        .expect_value_eq(expected)
}

#[test]
fn metadata_path_columns_multiple() -> Result {
    let expected = indoc! {"
        \u{1b}[39m╭───┬─────┬─────────╮\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m#\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[1;32mdir\u{1b}[0m \u{1b}[39m│\u{1b}[0m  \u{1b}[1;32mfile\u{1b}[0m   \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m├───┼─────┼─────────┤\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m0\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[39m\u{1b}[38;5;81msrc\u{1b}[0m\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[39m\u{1b}[38;5;48mmain.rs\u{1b}[0m\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m╰───┴─────┴─────────╯\u{1b}[0m
    "};
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.use_ansi_coloring = true
                $env.config.shell_integration.osc8 = false
                $data | metadata set --path-columns [dir file] | table
            ",
            test_value!([
                { dir: "src", file: "main.rs" },
            ]),
        )
        .expect_value_eq(expected)
}

#[test]
fn metadata_path_columns_multiple_with_icons() -> Result {
    let expected = indoc! {"
        \u{1b}[39m╭───┬────────┬────────────╮\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m#\u{1b}[0m \u{1b}[39m│\u{1b}[0m  \u{1b}[1;32mdir\u{1b}[0m   \u{1b}[39m│\u{1b}[0m    \u{1b}[1;32mfile\u{1b}[0m    \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m├───┼────────┼────────────┤\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m0\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[39m\u{1b}[38;2;126;142;168m\u{f115}\u{1b}[0m  \u{1b}[38;5;81msrc\u{1b}[0m\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[39m\u{1b}[38;2;222;165;132m\u{e68b}\u{1b}[0m  \u{1b}[38;5;48mmain.rs\u{1b}[0m\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m╰───┴────────┴────────────╯\u{1b}[0m
    "};
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.use_ansi_coloring = true
                $env.config.shell_integration.osc8 = false
                $data | metadata set --path-columns [dir file] | table --icons
            ",
            test_value!([
                { dir: "src", file: "main.rs" },
            ]),
        )
        .expect_value_eq(expected)
}
