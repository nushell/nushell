use indoc::indoc;
use nu_test_support::prelude::*;

#[test]
fn table_theme_on_border_light() -> Result {
    assert_eq!(
        create_theme_output("light")?,
        [
            "─#───a───b─────────c──────── 0   1   2                3  1   4   5   [list 3 items] ",
            "─#───a───b─────────c──────── 0   1   2                3  1   4   5   [list 3 items] ─#───a───b─────────c────────",
            "─#───a───b───c─ 0   1   2   3 ─#───a───b───c─",
            "─#───a_looooooong_name───b───c─ 0                   1   2   3 ─#───a_looooooong_name───b───c─",
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_basic() -> Result {
    assert_eq!(
        create_theme_output("basic")?,
        [
            "+-#-+-a-+-b-+-------c--------+| 0 | 1 | 2 |              3 |+---+---+---+----------------+| 1 | 4 | 5 | [list 3 items] |+---+---+---+----------------+",
            "+-#-+-a-+-b-+-------c--------+| 0 | 1 | 2 |              3 |+---+---+---+----------------+| 1 | 4 | 5 | [list 3 items] |+-#-+-a-+-b-+-------c--------+",
            "+-#-+-a-+-b-+-c-+| 0 | 1 | 2 | 3 |+-#-+-a-+-b-+-c-+",
            "+-#-+-a_looooooong_name-+-b-+-c-+| 0 |                 1 | 2 | 3 |+-#-+-a_looooooong_name-+-b-+-c-+"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_compact() -> Result {
    assert_eq!(
        create_theme_output("compact")?,
        [
            "─#─┬─a─┬─b─┬───────c──────── 0 │ 1 │ 2 │              3  1 │ 4 │ 5 │ [list 3 items] ───┴───┴───┴────────────────",
            "─#─┬─a─┬─b─┬───────c──────── 0 │ 1 │ 2 │              3  1 │ 4 │ 5 │ [list 3 items] ─#─┴─a─┴─b─┴───────c────────",
            "─#─┬─a─┬─b─┬─c─ 0 │ 1 │ 2 │ 3 ─#─┴─a─┴─b─┴─c─",
            "─#─┬─a_looooooong_name─┬─b─┬─c─ 0 │                 1 │ 2 │ 3 ─#─┴─a_looooooong_name─┴─b─┴─c─"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_frameless() -> Result {
    assert_eq!(
        create_theme_output("frameless")?,
        [
            "─#─┼─a─┼─b─┼───────c──────── 0 │ 1 │ 2 │              3  1 │ 4 │ 5 │ [list 3 items] ",
            "─#─┼─a─┼─b─┼───────c──────── 0 │ 1 │ 2 │              3  1 │ 4 │ 5 │ [list 3 items] ─#─┼─a─┼─b─┼───────c────────",
            "─#─┼─a─┼─b─┼─c─ 0 │ 1 │ 2 │ 3 ─#─┼─a─┼─b─┼─c─",
            "─#─┼─a_looooooong_name─┼─b─┼─c─ 0 │                 1 │ 2 │ 3 ─#─┼─a_looooooong_name─┼─b─┼─c─"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_compact_double() -> Result {
    assert_eq!(
        create_theme_output("compact_double")?,
        [
            "═#═╦═a═╦═b═╦═══════c════════ 0 ║ 1 ║ 2 ║              3  1 ║ 4 ║ 5 ║ [list 3 items] ═══╩═══╩═══╩════════════════",
            "═#═╦═a═╦═b═╦═══════c════════ 0 ║ 1 ║ 2 ║              3  1 ║ 4 ║ 5 ║ [list 3 items] ═#═╩═a═╩═b═╩═══════c════════",
            "═#═╦═a═╦═b═╦═c═ 0 ║ 1 ║ 2 ║ 3 ═#═╩═a═╩═b═╩═c═",
            "═#═╦═a_looooooong_name═╦═b═╦═c═ 0 ║                 1 ║ 2 ║ 3 ═#═╩═a_looooooong_name═╩═b═╩═c═"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_default() -> Result {
    assert_eq!(
        create_theme_output("default")?,
        [
            "╭─#─┬─a─┬─b─┬───────c────────╮│ 0 │ 1 │ 2 │              3 ││ 1 │ 4 │ 5 │ [list 3 items] │╰───┴───┴───┴────────────────╯",
            "╭─#─┬─a─┬─b─┬───────c────────╮│ 0 │ 1 │ 2 │              3 ││ 1 │ 4 │ 5 │ [list 3 items] │╰─#─┴─a─┴─b─┴───────c────────╯",
            "╭─#─┬─a─┬─b─┬─c─╮│ 0 │ 1 │ 2 │ 3 │╰─#─┴─a─┴─b─┴─c─╯",
            "╭─#─┬─a_looooooong_name─┬─b─┬─c─╮│ 0 │                 1 │ 2 │ 3 │╰─#─┴─a_looooooong_name─┴─b─┴─c─╯"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_heavy() -> Result {
    assert_eq!(
        create_theme_output("heavy")?,
        [
            "┏━#━┳━a━┳━b━┳━━━━━━━c━━━━━━━━┓┃ 0 ┃ 1 ┃ 2 ┃              3 ┃┃ 1 ┃ 4 ┃ 5 ┃ [list 3 items] ┃┗━━━┻━━━┻━━━┻━━━━━━━━━━━━━━━━┛",
            "┏━#━┳━a━┳━b━┳━━━━━━━c━━━━━━━━┓┃ 0 ┃ 1 ┃ 2 ┃              3 ┃┃ 1 ┃ 4 ┃ 5 ┃ [list 3 items] ┃┗━#━┻━a━┻━b━┻━━━━━━━c━━━━━━━━┛",
            "┏━#━┳━a━┳━b━┳━c━┓┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃┗━#━┻━a━┻━b━┻━c━┛",
            "┏━#━┳━a_looooooong_name━┳━b━┳━c━┓┃ 0 ┃                 1 ┃ 2 ┃ 3 ┃┗━#━┻━a_looooooong_name━┻━b━┻━c━┛"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_reinforced() -> Result {
    assert_eq!(
        create_theme_output("reinforced")?,
        [
            "┏─#─┬─a─┬─b─┬───────c────────┓│ 0 │ 1 │ 2 │              3 ││ 1 │ 4 │ 5 │ [list 3 items] │┗───┴───┴───┴────────────────┛",
            "┏─#─┬─a─┬─b─┬───────c────────┓│ 0 │ 1 │ 2 │              3 ││ 1 │ 4 │ 5 │ [list 3 items] │┗─#─┴─a─┴─b─┴───────c────────┛",
            "┏─#─┬─a─┬─b─┬─c─┓│ 0 │ 1 │ 2 │ 3 │┗─#─┴─a─┴─b─┴─c─┛",
            "┏─#─┬─a_looooooong_name─┬─b─┬─c─┓│ 0 │                 1 │ 2 │ 3 │┗─#─┴─a_looooooong_name─┴─b─┴─c─┛"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_none() -> Result {
    assert_eq!(
        create_theme_output("none")?,
        [
            " #   a   b         c         0   1   2                3  1   4   5   [list 3 items] ",
            " #   a   b         c         0   1   2                3  1   4   5   [list 3 items]  #   a   b         c        ",
            " #   a   b   c  0   1   2   3  #   a   b   c ",
            " #   a_looooooong_name   b   c  0                   1   2   3  #   a_looooooong_name   b   c "
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_rounded() -> Result {
    assert_eq!(
        create_theme_output("rounded")?,
        [
            "╭─#─┬─a─┬─b─┬───────c────────╮│ 0 │ 1 │ 2 │              3 ││ 1 │ 4 │ 5 │ [list 3 items] │╰───┴───┴───┴────────────────╯",
            "╭─#─┬─a─┬─b─┬───────c────────╮│ 0 │ 1 │ 2 │              3 ││ 1 │ 4 │ 5 │ [list 3 items] │╰─#─┴─a─┴─b─┴───────c────────╯",
            "╭─#─┬─a─┬─b─┬─c─╮│ 0 │ 1 │ 2 │ 3 │╰─#─┴─a─┴─b─┴─c─╯",
            "╭─#─┬─a_looooooong_name─┬─b─┬─c─╮│ 0 │                 1 │ 2 │ 3 │╰─#─┴─a_looooooong_name─┴─b─┴─c─╯"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_with_love() -> Result {
    assert_eq!(
        create_theme_output("with_love")?,
        [
            "❤#❤❤❤a❤❤❤b❤❤❤❤❤❤❤❤❤c❤❤❤❤❤❤❤❤ 0 ❤ 1 ❤ 2 ❤              3  1 ❤ 4 ❤ 5 ❤ [list 3 items] ❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
            "❤#❤❤❤a❤❤❤b❤❤❤❤❤❤❤❤❤c❤❤❤❤❤❤❤❤ 0 ❤ 1 ❤ 2 ❤              3  1 ❤ 4 ❤ 5 ❤ [list 3 items] ❤#❤❤❤a❤❤❤b❤❤❤❤❤❤❤❤❤c❤❤❤❤❤❤❤❤",
            "❤#❤❤❤a❤❤❤b❤❤❤c❤ 0 ❤ 1 ❤ 2 ❤ 3 ❤#❤❤❤a❤❤❤b❤❤❤c❤",
            "❤#❤❤❤a_looooooong_name❤❤❤b❤❤❤c❤ 0 ❤                 1 ❤ 2 ❤ 3 ❤#❤❤❤a_looooooong_name❤❤❤b❤❤❤c❤"
        ]
    );
    Ok(())
}

#[test]
fn table_theme_on_border_thin() -> Result {
    assert_eq!(
        create_theme_output("thin")?,
        // ["┌─#─┬a_looooooong_name┬─b─┬─c─┐│ 0 │               1 │ 2 │ 3 │└─#─┴a_looooooong_name┴─b─┴─c─┘"]
        [
            "┌─#─┬─a─┬─b─┬───────c────────┐│ 0 │ 1 │ 2 │              3 │├───┼───┼───┼────────────────┤│ 1 │ 4 │ 5 │ [list 3 items] │└───┴───┴───┴────────────────┘",
            "┌─#─┬─a─┬─b─┬───────c────────┐│ 0 │ 1 │ 2 │              3 │├───┼───┼───┼────────────────┤│ 1 │ 4 │ 5 │ [list 3 items] │└─#─┴─a─┴─b─┴───────c────────┘",
            "┌─#─┬─a─┬─b─┬─c─┐│ 0 │ 1 │ 2 │ 3 │└─#─┴─a─┴─b─┴─c─┘",
            "┌─#─┬─a_looooooong_name─┬─b─┬─c─┐│ 0 │                 1 │ 2 │ 3 │└─#─┴─a_looooooong_name─┴─b─┴─c─┘",
        ]
    );
    Ok(())
}

fn create_theme_output(theme: &str) -> Result<Vec<String>> {
    let mut tester = test();
    let normalize = |output: String| output.replace('\n', "");
    Ok(vec![
        normalize(tester.run_with_data(
            format!(
                "let data = $in\n{}",
                theme_cmd(theme, false, "$data | table --width=80")
            ),
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )?),
        normalize(tester.run_with_data(
            format!(
                "let data = $in\n{}",
                theme_cmd(theme, true, "$data | table --width=80")
            ),
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )?),
        normalize(tester.run_with_data(
            format!(
                "let data = $in\n{}",
                theme_cmd(theme, true, "$data | table --width=80")
            ),
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
            ],
        )?),
        normalize(tester.run_with_data(
            format!(
                "let data = $in\n{}",
                theme_cmd(theme, true, "$data | table --width=80")
            ),
            test_table![
                ["a_looooooong_name", "b", "c"];
                [1, 2, 3],
            ],
        )?),
    ])
}

fn theme_cmd(theme: &str, footer: bool, then: &str) -> String {
    let with_footer = if footer {
        "$env.config.footer_mode = \"always\"\n"
    } else {
        ""
    };
    format!(
        "$env.config.table.mode = \"{theme}\"\n$env.config.table.header_on_separator = true\n{with_footer}{then}"
    )
}

#[test]
fn table_theme_arg() -> Result {
    let mut tester = test();
    tester
        .run_with_data(
            "table --width=80 --theme light",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(indoc! {"
            \x20#   a   b         c        
            ────────────────────────────
            \x200   1   2                3 
            \x201   4   5   [list 3 items] 
            \x202   1   2                3 
        "})?;
    tester
        .run_with_data(
            format!(
                "let data = $in\n{}",
                theme_cmd("basic", false, "$data | table --width=80 --theme light")
            ),
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ─#───a───b─────────c────────
            \x200   1   2                3 
            \x201   4   5   [list 3 items] 
            \x202   1   2                3 
        "})
}

#[test]
fn table_markup_themes_ignore_header_on_separator() -> Result {
    let cmd = |theme: &str| {
        format!(
            "let data = $in\n$env.config.table.header_on_separator = true\n$data | table --width=80 --theme {theme}"
        )
    };
    let mut tester = test();

    tester
        .run_with_data(cmd("markdown"), test_table![["a", "b"]; [1, 2]])
        .expect_value_eq(indoc! {"
            | # | a | b |
            |---|---|---|
            | 0 | 1 | 2 |
        "})?;
    tester
        .run_with_data(cmd("restructured"), test_table![["a", "b"]; [1, 2]])
        .expect_value_eq(indoc! {"
            === === ===
            \x20#   a   b\x20
            === === ===
            \x200   1   2\x20
            === === ===
        "})
}
