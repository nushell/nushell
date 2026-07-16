use indoc::indoc;
use itertools::Itertools;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use rstest::rstest;

const WIDTH_PRIORITY_EIGHT_COL_INPUT: &str = indoc! {"
    [
        [a b c d e f g h];
        [value_0000000000000000 value_1111111111111111 value_2222222222222222 value_3333333333333333 value_4444444444444444 value_5555555555555555 priority_value_12345 value_7777777777777777]
        [value_0000000000000000 value_1111111111111111 value_2222222222222222 value_3333333333333333 value_4444444444444444 value_5555555555555555 priority_value_12345 value_7777777777777777]
    ]
"};
const WIDTH_PRIORITY_RECORD_INPUT: &str = indoc! {"
    [
        {c0: v00000000000 c1: v11111111111 c2: v22222222222 c3: v33333333333 c4: v44444444444 c5: v55555555555 c6: v66666666666 c7: v77777777777 c8: v88888888888 c9: v99999999999}
        {c0: v00000000000 c1: v11111111111 c2: v22222222222 c3: v33333333333 c4: v44444444444 c5: v55555555555 c6: v66666666666 c7: v77777777777 c8: v88888888888 c9: v99999999999}
    ]
"};
const WIDTH_PRIORITY_NAME_INPUT: &str = indoc! {"
    [
        [name type target readonly mode num_links inode user group size created accessed modified];
        [very_very_very_long_filename_that_should_get_priority_and_avoid_wrapping.txt file '' false rw-r--r-- 1 12345 me staff 1234 '2 years ago' '2 years ago' '2 years ago']
        [another_extremely_long_name_for_priority_column_display.txt file '' false rw-r--r-- 1 54321 me staff 5678 '2 years ago' '2 years ago' '2 years ago']
    ]
"};
const TABLE_CFG_HEADER_SEPARATOR: &str = "{ table: { header_on_separator: true } }";
const TABLE_CFG_BASIC_NO_INDEX: &str =
    "{ table: { mode: basic, index_mode: never, header_on_separator: false } }";
const TABLE_CFG_BASIC_WITH_INDEX: &str =
    "{ table: { mode: basic, index_mode: always, header_on_separator: false } }";

#[test]
fn table_0() -> Result {
    test()
        .run_with_data(
            "table --width=80",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬────────────────╮
            │ # │ a │ b │       c        │
            ├───┼───┼───┼────────────────┤
            │ 0 │ 1 │ 2 │              3 │
            │ 1 │ 4 │ 5 │ [list 3 items] │
            ╰───┴───┴───┴────────────────╯
        "})
}

#[test]
fn table_uses_metadata_width_priority_columns() -> Result {
    let without_priority =
        run_table_with_priority(WIDTH_PRIORITY_EIGHT_COL_INPUT, 121, None, None)?;

    let with_priority =
        run_table_with_priority(WIDTH_PRIORITY_EIGHT_COL_INPUT, 121, None, Some("g"))?;

    assert!(!without_priority.contains("priority_value_12345"));
    assert!(
        with_priority.contains("priority_value"),
        "expected prioritized column content to be visible; output was:\n{}",
        with_priority
    );

    Ok(())
}

#[test]
fn table_uses_metadata_width_priority_columns_with_header_separator() -> Result {
    let without_priority = run_table_with_priority(
        WIDTH_PRIORITY_EIGHT_COL_INPUT,
        121,
        Some(TABLE_CFG_HEADER_SEPARATOR),
        None,
    )?;

    let with_priority = run_table_with_priority(
        WIDTH_PRIORITY_EIGHT_COL_INPUT,
        121,
        Some(TABLE_CFG_HEADER_SEPARATOR),
        Some("g"),
    )?;

    assert!(!without_priority.contains("priority_value_12345"));
    assert!(with_priority.contains("priority_value_12345"));

    Ok(())
}

#[rstest]
#[case("c1", "c7")]
#[case("c0", "c9")]
#[case("c2", "c5")]
#[case("c3", "c8")]
#[case("c4", "c6")]
fn metadata_set_width_priority_columns_are_wider_than_other_columns(
    #[case] priority_one: &str,
    #[case] priority_two: &str,
) -> Result {
    let actual = run_table_with_priority(
        WIDTH_PRIORITY_RECORD_INPUT,
        135,
        Some(TABLE_CFG_BASIC_NO_INDEX),
        Some(&format!("{priority_one} {priority_two}")),
    )?;

    let widths = parse_basic_table_column_widths(&actual);
    assert_eq!(
        widths.len(),
        10,
        "expected all 10 columns visible for width comparison; output was:\n{}",
        actual
    );

    let p1 = parse_c_column_index(priority_one);
    let p2 = parse_c_column_index(priority_two);
    let p1_width = widths[p1];
    let p2_width = widths[p2];

    for (i, &width) in widths.iter().enumerate() {
        if i == p1 || i == p2 {
            continue;
        }

        assert!(
            p1_width > width,
            "expected {priority_one} width ({p1_width}) to be greater than c{i} width ({width}); output was:\n{}",
            actual
        );
        assert!(
            p2_width > width,
            "expected {priority_two} width ({p2_width}) to be greater than c{i} width ({width}); output was:\n{}",
            actual
        );
    }

    Ok(())
}

#[test]
fn single_name_priority_drops_trailing_columns_for_long_values() -> Result {
    let actual = run_table_with_priority(
        WIDTH_PRIORITY_NAME_INPUT,
        140,
        Some(TABLE_CFG_BASIC_WITH_INDEX),
        Some("name"),
    )?;

    assert!(
        actual.contains(
            "very_very_very_long_filename_that_should_get_priority_and_avoid_wrapping.txt"
        ),
        "expected full prioritized name to be visible; output was:\n{}",
        actual
    );
    assert!(
        !actual.contains("created"),
        "expected trailing columns to be collapsed; output was:\n{}",
        actual
    );

    Ok(())
}

/// Parses table header cell widths for the current rendering mode.
fn parse_basic_table_column_widths(output: &str) -> Vec<usize> {
    let header_line = output
        .lines()
        .find(|line| line.contains("c0") && line.contains("c1"))
        .expect("expected header line with test columns");

    let separator = if header_line.contains('┃') {
        '┃'
    } else if header_line.contains('│') {
        '│'
    } else {
        '|'
    };

    let cells: Vec<&str> = header_line.split(separator).collect();
    cells[1..cells.len() - 1]
        .iter()
        .map(|cell| cell.chars().count())
        .collect()
}

/// Runs `table` with optional `$env.config` setup and optional metadata width-priority columns.
fn run_table_with_priority(
    input: &str,
    width: usize,
    table_config: Option<&str>,
    priority_columns: Option<&str>,
) -> Result<String> {
    let mut pipeline = String::from(input);

    if let Some(priority_columns) = priority_columns {
        pipeline.push_str(" | metadata set --table-width-priority-columns [");
        pipeline.push_str(priority_columns);
        pipeline.push(']');
    }

    pipeline.push_str(" | table --width ");
    pipeline.push_str(&width.to_string());

    let code = if let Some(table_config) = table_config {
        format!("$env.config = {table_config}\n{pipeline}")
    } else {
        pipeline
    };

    test().run(&code)
}

fn parse_c_column_index(column: &str) -> usize {
    // Columns in this test use the fixed naming scheme `cN`.
    let number = column
        .strip_prefix('c')
        .expect("expected column name format cN");

    number.parse::<usize>().expect("expected numeric suffix")
}

#[test]
fn table_collapse_0() -> Result {
    test()
        .run_with_data(
            "table --width=80 --collapse",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───╮
            │ a │ b │ c │
            ├───┼───┼───┤
            │ 1 │ 2 │ 3 │
            ├───┼───┼───┤
            │ 4 │ 5 │ 1 │
            │   │   ├───┤
            │   │   │ 2 │
            │   │   ├───┤
            │   │   │ 3 │
            ╰───┴───┴───╯
        "})
}

#[test]
fn table_collapse_basic() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: basic }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            +---+---+---+
            | a | b | c |
            +---+---+---+
            | 1 | 2 | 3 |
            +---+---+---+
            | 4 | 5 | 1 |
            |   |   +---+
            |   |   | 2 |
            |   |   +---+
            |   |   | 3 |
            +---+---+---+
        "})
}

#[test]
fn table_collapse_heavy() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: heavy }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ┏━━━┳━━━┳━━━┓
            ┃ a ┃ b ┃ c ┃
            ┣━━━╋━━━╋━━━┫
            ┃ 1 ┃ 2 ┃ 3 ┃
            ┣━━━╋━━━╋━━━┫
            ┃ 4 ┃ 5 ┃ 1 ┃
            ┃   ┃   ┣━━━┫
            ┃   ┃   ┃ 2 ┃
            ┃   ┃   ┣━━━┫
            ┃   ┃   ┃ 3 ┃
            ┗━━━┻━━━┻━━━┛
        "})
}

#[test]
fn table_collapse_compact() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: compact }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ┌───┬───┬───┐
            │ a │ b │ c │
            ├───┼───┼───┤
            │ 1 │ 2 │ 3 │
            ├───┼───┼───┤
            │ 4 │ 5 │ 1 │
            │   │   ├───┤
            │   │   │ 2 │
            │   │   ├───┤
            │   │   │ 3 │
            └───┴───┴───┘
        "})
}

#[test]
fn table_collapse_compact_double() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: compact_double }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╔═══╦═══╦═══╗
            ║ a ║ b ║ c ║
            ╠═══╬═══╬═══╣
            ║ 1 ║ 2 ║ 3 ║
            ╠═══╬═══╬═══╣
            ║ 4 ║ 5 ║ 1 ║
            ║   ║   ╠═══╣
            ║   ║   ║ 2 ║
            ║   ║   ╠═══╣
            ║   ║   ║ 3 ║
            ╚═══╩═══╩═══╝
        "})
}

#[test]
fn table_collapse_compact_light() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: light }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ┌───┬───┬───┐
            │ a │ b │ c │
            ├───┼───┼───┤
            │ 1 │ 2 │ 3 │
            ├───┼───┼───┤
            │ 4 │ 5 │ 1 │
            │   │   ├───┤
            │   │   │ 2 │
            │   │   ├───┤
            │   │   │ 3 │
            └───┴───┴───┘
        "})
}

#[test]
fn table_collapse_none() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: none }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            \x20a  b  c 
            \x201  2  3 
            \x204  5  1 
            \x20      2 
            \x20      3 
        "})
}

#[test]
fn table_collapse_compact_reinforced() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: reinforced }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ┏───┬───┬───┓
            │ a │ b │ c │
            ├───┼───┼───┤
            │ 1 │ 2 │ 3 │
            ├───┼───┼───┤
            │ 4 │ 5 │ 1 │
            │   │   ├───┤
            │   │   │ 2 │
            │   │   ├───┤
            │   │   │ 3 │
            ┗───┴───┴───┛
        "})
}

#[test]
fn table_collapse_compact_thin() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: thin }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ┌───┬───┬───┐
            │ a │ b │ c │
            ├───┼───┼───┤
            │ 1 │ 2 │ 3 │
            ├───┼───┼───┤
            │ 4 │ 5 │ 1 │
            │   │   ├───┤
            │   │   │ 2 │
            │   │   ├───┤
            │   │   │ 3 │
            └───┴───┴───┘
        "})
}

#[test]
fn table_collapse_hearts() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config = { table: { mode: with_love }}
                $data | table --width=80 --collapse
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ❤❤❤❤❤❤❤❤❤❤❤❤❤
            ❤ a ❤ b ❤ c ❤
            ❤❤❤❤❤❤❤❤❤❤❤❤❤
            ❤ 1 ❤ 2 ❤ 3 ❤
            ❤❤❤❤❤❤❤❤❤❤❤❤❤
            ❤ 4 ❤ 5 ❤ 1 ❤
            ❤   ❤   ❤❤❤❤❤
            ❤   ❤   ❤ 2 ❤
            ❤   ❤   ❤❤❤❤❤
            ❤   ❤   ❤ 3 ❤
            ❤❤❤❤❤❤❤❤❤❤❤❤❤
        "})
}

#[test]
fn table_collapse_does_wrapping_for_long_strings() -> Result {
    test()
        .run("
            [[a]; [11111111111111111111111111111111111111111111111111111111111111111111111111111111]]
            | table --width=80 --collapse
        ")
        .expect_value_eq(indoc! {"
            ╭────────────────────────────────╮
            │ a                              │
            ├────────────────────────────────┤
            │ 111111111111111109312339230430 │
            │ 179149313814687359833671239329 │
            │ 01313323321729744896.00        │
            ╰────────────────────────────────╯
        "})
}

#[test]
fn table_expand_0() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬───────────╮
            │ # │ a │ b │     c     │
            ├───┼───┼───┼───────────┤
            │ 0 │ 1 │ 2 │         3 │
            │ 1 │ 4 │ 5 │ ╭───┬───╮ │
            │   │   │   │ │ 0 │ 1 │ │
            │   │   │   │ │ 1 │ 2 │ │
            │   │   │   │ │ 2 │ 3 │ │
            │   │   │   │ ╰───┴───╯ │
            ╰───┴───┴───┴───────────╯
        "})
}

// I am not sure whether the test is platform dependent, cause we don't set a term_width on our own
#[test]
fn table_expand_exceed_overlap_0() -> Result {
    // no expand

    test()
        .run_with_data(
            "table --width=80 --expand",
            test_table![
                ["a", "b", "c"];
                ["xxxxxxxxxxxxxxxxxxxxxx", 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬────────────────────────┬───┬───────────╮
            │ # │           a            │ b │     c     │
            ├───┼────────────────────────┼───┼───────────┤
            │ 0 │ xxxxxxxxxxxxxxxxxxxxxx │ 2 │         3 │
            │ 1 │                      4 │ 5 │ ╭───┬───╮ │
            │   │                        │   │ │ 0 │ 1 │ │
            │   │                        │   │ │ 1 │ 2 │ │
            │   │                        │   │ │ 2 │ 3 │ │
            │   │                        │   │ ╰───┴───╯ │
            ╰───┴────────────────────────┴───┴───────────╯
        "})?;

    // expand

    test()
        .run_with_data(
            "table --width=80 --expand",
            test_table![
                ["a", "b", "c"];
                ["xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭──────┬───────────────────────────────────────────────────┬─────┬─────────────╮
            │    # │                         a                         │  b  │      c      │
            ├──────┼───────────────────────────────────────────────────┼─────┼─────────────┤
            │    0 │ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx    │   2 │           3 │
            │    1 │                                                 4 │   5 │ ╭───┬───╮   │
            │      │                                                   │     │ │ 0 │ 1 │   │
            │      │                                                   │     │ │ 1 │ 2 │   │
            │      │                                                   │     │ │ 2 │ 3 │   │
            │      │                                                   │     │ ╰───┴───╯   │
            ╰──────┴───────────────────────────────────────────────────┴─────┴─────────────╯
        "})
}

#[test]
fn table_expand_deep_0() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand --expand-deep=1",
            test_value!([{ a: 1, b: 2, c: 3 }, { a: 4, b: 5, c: [1, 2, [1, 2, 3]] }]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬────────────────────────╮
            │ # │ a │ b │           c            │
            ├───┼───┼───┼────────────────────────┤
            │ 0 │ 1 │ 2 │                      3 │
            │ 1 │ 4 │ 5 │ ╭───┬────────────────╮ │
            │   │   │   │ │ 0 │              1 │ │
            │   │   │   │ │ 1 │              2 │ │
            │   │   │   │ │ 2 │ [list 3 items] │ │
            │   │   │   │ ╰───┴────────────────╯ │
            ╰───┴───┴───┴────────────────────────╯
        "})
}

#[test]
fn table_expand_deep_1() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand --expand-deep=0",
            test_value!([{ a: 1, b: 2, c: 3 }, { a: 4, b: 5, c: [1, 2, [1, 2, 3]] }]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬────────────────╮
            │ # │ a │ b │       c        │
            ├───┼───┼───┼────────────────┤
            │ 0 │ 1 │ 2 │              3 │
            │ 1 │ 4 │ 5 │ [list 3 items] │
            ╰───┴───┴───┴────────────────╯
        "})
}

#[test]
fn table_expand_flatten_0() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand --flatten",
            test_value!([{ a: 1, b: 2, c: 3 }, { a: 4, b: 5, c: [1, 2, [1, 1, 1]] }]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬───────────────╮
            │ # │ a │ b │       c       │
            ├───┼───┼───┼───────────────┤
            │ 0 │ 1 │ 2 │             3 │
            │ 1 │ 4 │ 5 │ ╭───┬───────╮ │
            │   │   │   │ │ 0 │     1 │ │
            │   │   │   │ │ 1 │     2 │ │
            │   │   │   │ │ 2 │ 1 1 1 │ │
            │   │   │   │ ╰───┴───────╯ │
            ╰───┴───┴───┴───────────────╯
        "})
}

#[test]
fn table_expand_flatten_1() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand --flatten --flatten-separator=,",
            test_value!([{ a: 1, b: 2, c: 3 }, { a: 4, b: 5, c: [1, 2, [1, 1, 1]] }]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬───────────────╮
            │ # │ a │ b │       c       │
            ├───┼───┼───┼───────────────┤
            │ 0 │ 1 │ 2 │             3 │
            │ 1 │ 4 │ 5 │ ╭───┬───────╮ │
            │   │   │   │ │ 0 │     1 │ │
            │   │   │   │ │ 1 │     2 │ │
            │   │   │   │ │ 2 │ 1,1,1 │ │
            │   │   │   │ ╰───┴───────╯ │
            ╰───┴───┴───┴───────────────╯
        "})
}

#[test]
fn table_expand_flatten_and_deep_1() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand --expand-deep=2 --flatten --flatten-separator=,",
            test_value!([
                { a: 1, b: 2, c: 3 },
                { a: 4, b: 5, c: [1, 2, [1, [1, 1, 1], 1]] }
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬────────────────────────────────╮
            │ # │ a │ b │               c                │
            ├───┼───┼───┼────────────────────────────────┤
            │ 0 │ 1 │ 2 │                              3 │
            │ 1 │ 4 │ 5 │ ╭───┬────────────────────────╮ │
            │   │   │   │ │ 0 │                      1 │ │
            │   │   │   │ │ 1 │                      2 │ │
            │   │   │   │ │ 2 │ ╭───┬────────────────╮ │ │
            │   │   │   │ │   │ │ 0 │              1 │ │ │
            │   │   │   │ │   │ │ 1 │ [list 3 items] │ │ │
            │   │   │   │ │   │ │ 2 │              1 │ │ │
            │   │   │   │ │   │ ╰───┴────────────────╯ │ │
            │   │   │   │ ╰───┴────────────────────────╯ │
            ╰───┴───┴───┴────────────────────────────────╯
        "})
}

#[test]
fn table_expand_record_0() -> Result {
    test()
        .run_with_data("table --width=80 --expand", [test_value!({ c: { d: 1 } })])
        .expect_value_eq(indoc! {"
            ╭───┬───────────╮
            │ # │     c     │
            ├───┼───────────┤
            │ 0 │ ╭───┬───╮ │
            │   │ │ d │ 1 │ │
            │   │ ╰───┴───╯ │
            ╰───┴───────────╯
        "})
}

#[test]
fn table_expand_record_1() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand",
            test_value!([
                { a: 1, b: 2, c: 3 },
                { a: 4, b: 5, c: [1, 2, { a: 123, b: 234, c: 345 }] }
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬─────────────────────╮
            │ # │ a │ b │          c          │
            ├───┼───┼───┼─────────────────────┤
            │ 0 │ 1 │ 2 │                   3 │
            │ 1 │ 4 │ 5 │ ╭───┬─────────────╮ │
            │   │   │   │ │ 0 │           1 │ │
            │   │   │   │ │ 1 │           2 │ │
            │   │   │   │ │ 2 │ ╭───┬─────╮ │ │
            │   │   │   │ │   │ │ a │ 123 │ │ │
            │   │   │   │ │   │ │ b │ 234 │ │ │
            │   │   │   │ │   │ │ c │ 345 │ │ │
            │   │   │   │ │   │ ╰───┴─────╯ │ │
            │   │   │   │ ╰───┴─────────────╯ │
            ╰───┴───┴───┴─────────────────────╯
        "})
}

#[test]
fn table_expand_record_2() -> Result {
    let field3 = test_table![
        ["head1", "head2", "head3"];
        [1, 2, 3],
        [79, 79, 79],
        [test_value!({ f1: "a string", f2: 1000 }), 1, 2],
    ];

    test()
        .run_with_data(
            "table --width=80 --expand",
            test_value!({
                field1: ["a", "b", "c"],
                field2: [123, 234, 345],
                field3: (field3),
                field4: { f1: 1, f2: 3, f3: { f1: "f1", f2: "f2", f3: "f3" } }
            }),
        )
        .expect_value_eq(indoc! {"
            ╭────────┬───────────────────────────────────────────╮
            │        │ ╭───┬───╮                                 │
            │ field1 │ │ 0 │ a │                                 │
            │        │ │ 1 │ b │                                 │
            │        │ │ 2 │ c │                                 │
            │        │ ╰───┴───╯                                 │
            │        │ ╭───┬─────╮                               │
            │ field2 │ │ 0 │ 123 │                               │
            │        │ │ 1 │ 234 │                               │
            │        │ │ 2 │ 345 │                               │
            │        │ ╰───┴─────╯                               │
            │        │ ╭───┬───────────────────┬───────┬───────╮ │
            │ field3 │ │ # │       head1       │ head2 │ head3 │ │
            │        │ ├───┼───────────────────┼───────┼───────┤ │
            │        │ │ 0 │                 1 │     2 │     3 │ │
            │        │ │ 1 │                79 │    79 │    79 │ │
            │        │ │ 2 │ ╭────┬──────────╮ │     1 │     2 │ │
            │        │ │   │ │ f1 │ a string │ │       │       │ │
            │        │ │   │ │ f2 │ 1000     │ │       │       │ │
            │        │ │   │ ╰────┴──────────╯ │       │       │ │
            │        │ ╰───┴───────────────────┴───────┴───────╯ │
            │        │ ╭────┬─────────────╮                      │
            │ field4 │ │ f1 │ 1           │                      │
            │        │ │ f2 │ 3           │                      │
            │        │ │    │ ╭────┬────╮ │                      │
            │        │ │ f3 │ │ f1 │ f1 │ │                      │
            │        │ │    │ │ f2 │ f2 │ │                      │
            │        │ │    │ │ f3 │ f3 │ │                      │
            │        │ │    │ ╰────┴────╯ │                      │
            │        │ ╰────┴─────────────╯                      │
            ╰────────┴───────────────────────────────────────────╯"})
}

#[test]
#[cfg(not(windows))]
fn external_with_too_much_stdout_should_not_hang_nu() -> Result {
    use nu_test_support::fs::Stub::FileWithContent;

    use nu_test_support::playground::Playground;
    Playground::setup("external with too much stdout", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(&[FileWithContent("a_large_file.txt", &large_file_body)]);

        let actual: String = test().cwd(dirs.test()).run(
            "
            cat a_large_file.txt | table --width=80
        ",
        )?;

        assert_eq!(actual, large_file_body);

        let actual: String = test()
            .cwd(dirs.test())
            .run("let x = cat a_large_file.txt; $x")?;
        assert_eq!(actual, large_file_body);
        Ok(())
    })
}

#[test]
fn table_pagging_row_offset_overlap() -> Result {
    let actual: String = test().run("0..1000 | table")?;
    let rows = (0..1000)
        .map(|i| format!("│{i:>4} │{i:>4} │"))
        .collect::<Vec<_>>()
        .join("\n");
    let expected = format!(
        "{}\n{}\n{}",
        "╭─────┬─────╮",
        rows,
        indoc! {"
            ╰─────┴─────╯
            ╭──────┬──────╮
            │ 1000 │ 1000 │
            ╰──────┴──────╯
        "}
    );
    assert_eq!(actual, expected);
    Ok(())
}
#[test]
fn table_index_0() -> Result {
    let actual: String = test().run("[1 3 1 3 2 1 1] | table")?;
    assert_eq!(
        actual,
        indoc! {"
            ╭───┬───╮
            │ 0 │ 1 │
            │ 1 │ 3 │
            │ 2 │ 1 │
            │ 3 │ 3 │
            │ 4 │ 2 │
            │ 5 │ 1 │
            │ 6 │ 1 │
            ╰───┴───╯
        "}
    );
    Ok(())
}

#[test]
fn test_expand_big_0() -> Result {
    Playground::setup("test_expand_big_0", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
            [package]
            authors = ["The Nushell Project Developers"]
            default-run = "nu"
            description = "A new type of shell"
            documentation = "https://www.nushell.sh/book/"
            edition = "2024"
            exclude = ["images"]
            homepage = "https://www.nushell.sh"
            license = "MIT"
            name = "nu"
            repository = "https://github.com/nushell/nushell"
            rust-version = "1.60"
            version = "0.74.1"

            # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

            [package.metadata.binstall]
            pkg-url = "{ repo }/releases/download/{ version }/{ name }-{ version }-{ target }.{ archive-format }"
            pkg-fmt = "tgz"

            [package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
            pkg-fmt = "zip"

            [workspace]
            members = [
                "crates/nu-cli",
                "crates/nu-engine",
                "crates/nu-parser",
                "crates/nu-system",
                "crates/nu-command",
                "crates/nu-protocol",
                "crates/nu-plugin",
                "crates/nu_plugin_inc",
                "crates/nu_plugin_gstat",
                "crates/nu_plugin_example",
                "crates/nu_plugin_query",
                "crates/nu_plugin_custom_values",
                "crates/nu-utils",
            ]

            [dependencies]
            chrono = { version = "0.4.23", features = ["serde"] }
            crossterm = "0.24.0"
            ctrlc = "3.2.1"
            log = "0.4"
            miette = { version = "5.5.0", features = ["fancy-no-backtrace"] }
            nu-ansi-term = "0.46.0"
            nu-cli = { path = "./crates/nu-cli", version = "0.74.1" }
            nu-engine = { path = "./crates/nu-engine", version = "0.74.1" }
            reedline = { version = "0.14.0", features = ["bashisms", "sqlite"] }

            rayon = "1.6.1"
            is_executable = "1.0.1"
            simplelog = "0.12.0"
            time = "0.3.12"

            [target.'cfg(not(target_os = "windows"))'.dependencies]
            # Our dependencies don't use OpenSSL on Windows
            openssl = { version = "0.10.38", features = ["vendored"], optional = true }
            signal-hook = { version = "0.3.14", default-features = false }


            [target.'cfg(windows)'.build-dependencies]
            winres = "0.1"

            [target.'cfg(target_family = "unix")'.dependencies]
            nix = { version = "0.25", default-features = false, features = ["signal", "process", "fs", "term"] }
            atty = "0.2"

            [dev-dependencies]
            nu-test-support = { path = "./crates/nu-test-support", version = "0.74.1" }
            tempfile = "3.2.0"
            assert_cmd = "2.0.2"
            criterion = "0.4"
            pretty_assertions = "1.0.0"
            serial_test = "0.10.0"
            hamcrest2 = "0.3.0"
            rstest = { version = "0.15.0", default-features = false }
            itertools = "0.10.3"

            [features]
            plugin = [
                "nu-plugin",
                "nu-cli/plugin",
                "nu-parser/plugin",
                "nu-command/plugin",
                "nu-protocol/plugin",
                "nu-engine/plugin",
            ]
            # extra used to be more useful but now it's the same as default. Leaving it in for backcompat with existing build scripts
            extra = ["default"]
            default = ["plugin", "which-support", "trash-support", "sqlite"]
            stable = ["default"]
            wasi = []

            # Enable to statically link OpenSSL; otherwise the system version will be used. Not enabled by default because it takes a while to build
            static-link-openssl = ["dep:openssl"]

            # Stable (Default)
            which-support = ["nu-command/which-support"]
            trash-support = ["nu-command/trash-support"]

            # Main nu binary
            [[bin]]
            name = "nu"
            path = "src/main.rs"

            # To use a development version of a dependency please use a global override here
            # changing versions in each sub-crate of the workspace is tedious
            [patch.crates-io]
            reedline = { git = "https://github.com/nushell/reedline.git", branch = "main" }

            # Criterion benchmarking setup
            # Run all benchmarks with `cargo bench`
            # Run individual benchmarks like `cargo bench -- <regex>` e.g. `cargo bench -- parse`
            [[bench]]
            name = "benchmarks"
            harness = false
            "#,
        )]);

        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --width=80 --expand")?;

        let expected = indoc! {r#"
            ╭──────────────────┬───────────────────────────────────────────────────────────╮
            │                  │ ╭───────────────┬───────────────────────────────────────╮ │
            │ package          │ │               │ ╭───┬──────────────────────╮          │ │
            │                  │ │ authors       │ │ 0 │ The Nushell Project  │          │ │
            │                  │ │               │ │   │ Developers           │          │ │
            │                  │ │               │ ╰───┴──────────────────────╯          │ │
            │                  │ │ default-run   │ nu                                    │ │
            │                  │ │ description   │ A new type of shell                   │ │
            │                  │ │ documentation │ https://www.nushell.sh/book/          │ │
            │                  │ │ edition       │ 2024                                  │ │
            │                  │ │               │ ╭───┬────────╮                        │ │
            │                  │ │ exclude       │ │ 0 │ images │                        │ │
            │                  │ │               │ ╰───┴────────╯                        │ │
            │                  │ │ homepage      │ https://www.nushell.sh                │ │
            │                  │ │ license       │ MIT                                   │ │
            │                  │ │ name          │ nu                                    │ │
            │                  │ │ repository    │ https://github.com/nushell/nushell    │ │
            │                  │ │ rust-version  │ 1.60                                  │ │
            │                  │ │ version       │ 0.74.1                                │ │
            │                  │ │               │ ╭──────────┬────────────────────────╮ │ │
            │                  │ │ metadata      │ │          │ ╭───────────┬────────╮ │ │ │
            │                  │ │               │ │ binstall │ │ pkg-url   │ { repo │ │ │ │
            │                  │ │               │ │          │ │           │  }/rel │ │ │ │
            │                  │ │               │ │          │ │           │ eases/ │ │ │ │
            │                  │ │               │ │          │ │           │ downlo │ │ │ │
            │                  │ │               │ │          │ │           │ ad/{ v │ │ │ │
            │                  │ │               │ │          │ │           │ ersion │ │ │ │
            │                  │ │               │ │          │ │           │  }/{   │ │ │ │
            │                  │ │               │ │          │ │           │ name   │ │ │ │
            │                  │ │               │ │          │ │           │ }-{ ve │ │ │ │
            │                  │ │               │ │          │ │           │ rsion  │ │ │ │
            │                  │ │               │ │          │ │           │ }-{    │ │ │ │
            │                  │ │               │ │          │ │           │ target │ │ │ │
            │                  │ │               │ │          │ │           │  }.{ a │ │ │ │
            │                  │ │               │ │          │ │           │ rchive │ │ │ │
            │                  │ │               │ │          │ │           │ -forma │ │ │ │
            │                  │ │               │ │          │ │           │ t }    │ │ │ │
            │                  │ │               │ │          │ │ pkg-fmt   │ tgz    │ │ │ │
            │                  │ │               │ │          │ │ overrides │ {recor │ │ │ │
            │                  │ │               │ │          │ │           │ d 1    │ │ │ │
            │                  │ │               │ │          │ │           │ field} │ │ │ │
            │                  │ │               │ │          │ ╰───────────┴────────╯ │ │ │
            │                  │ │               │ ╰──────────┴────────────────────────╯ │ │
            │                  │ ╰───────────────┴───────────────────────────────────────╯ │
            │                  │ ╭───────────┬───────────────────────────────────────────╮ │
            │ workspace        │ │           │ ╭────┬────────────────────────────────╮   │ │
            │                  │ │ members   │ │  0 │ crates/nu-cli                  │   │ │
            │                  │ │           │ │  1 │ crates/nu-engine               │   │ │
            │                  │ │           │ │  2 │ crates/nu-parser               │   │ │
            │                  │ │           │ │  3 │ crates/nu-system               │   │ │
            │                  │ │           │ │  4 │ crates/nu-command              │   │ │
            │                  │ │           │ │  5 │ crates/nu-protocol             │   │ │
            │                  │ │           │ │  6 │ crates/nu-plugin               │   │ │
            │                  │ │           │ │  7 │ crates/nu_plugin_inc           │   │ │
            │                  │ │           │ │  8 │ crates/nu_plugin_gstat         │   │ │
            │                  │ │           │ │  9 │ crates/nu_plugin_example       │   │ │
            │                  │ │           │ │ 10 │ crates/nu_plugin_query         │   │ │
            │                  │ │           │ │ 11 │ crates/nu_plugin_custom_values │   │ │
            │                  │ │           │ │ 12 │ crates/nu-utils                │   │ │
            │                  │ │           │ ╰────┴────────────────────────────────╯   │ │
            │                  │ ╰───────────┴───────────────────────────────────────────╯ │
            │                  │ ╭───────────────┬───────────────────────────────────────╮ │
            │ dependencies     │ │               │ ╭──────────┬───────────────╮          │ │
            │                  │ │ chrono        │ │ version  │ 0.4.23        │          │ │
            │                  │ │               │ │          │ ╭───┬───────╮ │          │ │
            │                  │ │               │ │ features │ │ 0 │ serde │ │          │ │
            │                  │ │               │ │          │ ╰───┴───────╯ │          │ │
            │                  │ │               │ ╰──────────┴───────────────╯          │ │
            │                  │ │ crossterm     │ 0.24.0                                │ │
            │                  │ │ ctrlc         │ 3.2.1                                 │ │
            │                  │ │ log           │ 0.4                                   │ │
            │                  │ │               │ ╭──────────┬────────────────────────╮ │ │
            │                  │ │ miette        │ │ version  │ 5.5.0                  │ │ │
            │                  │ │               │ │          │ ╭───┬────────────────╮ │ │ │
            │                  │ │               │ │ features │ │ 0 │ fancy-no-backt │ │ │ │
            │                  │ │               │ │          │ │   │ race           │ │ │ │
            │                  │ │               │ │          │ ╰───┴────────────────╯ │ │ │
            │                  │ │               │ ╰──────────┴────────────────────────╯ │ │
            │                  │ │ nu-ansi-term  │ 0.46.0                                │ │
            │                  │ │               │ ╭─────────┬─────────────────╮         │ │
            │                  │ │ nu-cli        │ │ path    │ ./crates/nu-cli │         │ │
            │                  │ │               │ │ version │ 0.74.1          │         │ │
            │                  │ │               │ ╰─────────┴─────────────────╯         │ │
            │                  │ │               │ ╭────────────┬──────────────────────╮ │ │
            │                  │ │ nu-engine     │ │ path       │ ./crates/nu-engine   │ │ │
            │                  │ │               │ │ version    │ 0.74.1               │ │ │
            │                  │ │               │ ╰────────────┴──────────────────────╯ │ │
            │                  │ │               │ ╭─────────────┬─────────────────────╮ │ │
            │                  │ │ reedline      │ │ version     │ 0.14.0              │ │ │
            │                  │ │               │ │             │ ╭───┬──────────╮    │ │ │
            │                  │ │               │ │ features    │ │ 0 │ bashisms │    │ │ │
            │                  │ │               │ │             │ │ 1 │ sqlite   │    │ │ │
            │                  │ │               │ │             │ ╰───┴──────────╯    │ │ │
            │                  │ │               │ ╰─────────────┴─────────────────────╯ │ │
            │                  │ │ rayon         │ 1.6.1                                 │ │
            │                  │ │ is_executable │ 1.0.1                                 │ │
            │                  │ │ simplelog     │ 0.12.0                                │ │
            │                  │ │ time          │ 0.3.12                                │ │
            │                  │ ╰───────────────┴───────────────────────────────────────╯ │
            │                  │ ╭─────────────────────────────────┬─────────────────────╮ │
            │ target           │ │                                 │ ╭──────────────┬──╮ │ │
            │                  │ │ cfg(not(target_os = "windows")) │ │ dependencies │  │ │ │
            │                  │ │                                 │ ╰──────────────┴──╯ │ │
            │                  │ │ cfg(windows)                    │ {record 1 field}    │ │
            │                  │ │                                 │ ╭──────────────┬──╮ │ │
            │                  │ │ cfg(target_family = "unix")     │ │ dependencies │  │ │ │
            │                  │ │                                 │ ╰──────────────┴──╯ │ │
            │                  │ ╰─────────────────────────────────┴─────────────────────╯ │
            │                  │ ╭───────────────────┬───────────────────────────────────╮ │
            │ dev-dependencies │ │                   │ ╭─────────┬─────────────────────╮ │ │
            │                  │ │ nu-test-support   │ │ path    │ ./crates/nu-test-su │ │ │
            │                  │ │                   │ │         │ pport               │ │ │
            │                  │ │                   │ │ version │ 0.74.1              │ │ │
            │                  │ │                   │ ╰─────────┴─────────────────────╯ │ │
            │                  │ │ tempfile          │ 3.2.0                             │ │
            │                  │ │ assert_cmd        │ 2.0.2                             │ │
            │                  │ │ criterion         │ 0.4                               │ │
            │                  │ │ pretty_assertions │ 1.0.0                             │ │
            │                  │ │ serial_test       │ 0.10.0                            │ │
            │                  │ │ hamcrest2         │ 0.3.0                             │ │
            │                  │ │                   │ ╭────────────────────┬──────────╮ │ │
            │                  │ │ rstest            │ │ version            │ 0.15.0   │ │ │
            │                  │ │                   │ │ default-features   │ false    │ │ │
            │                  │ │                   │ ╰────────────────────┴──────────╯ │ │
            │                  │ │ itertools         │ 0.10.3                            │ │
            │                  │ ╰───────────────────┴───────────────────────────────────╯ │
            │                  │ ╭─────────────────────┬─────────────────────────────────╮ │
            │ features         │ │                     │ ╭───┬────────────────────╮      │ │
            │                  │ │ plugin              │ │ 0 │ nu-plugin          │      │ │
            │                  │ │                     │ │ 1 │ nu-cli/plugin      │      │ │
            │                  │ │                     │ │ 2 │ nu-parser/plugin   │      │ │
            │                  │ │                     │ │ 3 │ nu-command/plugin  │      │ │
            │                  │ │                     │ │ 4 │ nu-protocol/plugin │      │ │
            │                  │ │                     │ │ 5 │ nu-engine/plugin   │      │ │
            │                  │ │                     │ ╰───┴────────────────────╯      │ │
            │                  │ │                     │ ╭───┬─────────╮                 │ │
            │                  │ │ extra               │ │ 0 │ default │                 │ │
            │                  │ │                     │ ╰───┴─────────╯                 │ │
            │                  │ │                     │ ╭───┬───────────────╮           │ │
            │                  │ │ default             │ │ 0 │ plugin        │           │ │
            │                  │ │                     │ │ 1 │ which-support │           │ │
            │                  │ │                     │ │ 2 │ trash-support │           │ │
            │                  │ │                     │ │ 3 │ sqlite        │           │ │
            │                  │ │                     │ ╰───┴───────────────╯           │ │
            │                  │ │                     │ ╭───┬─────────╮                 │ │
            │                  │ │ stable              │ │ 0 │ default │                 │ │
            │                  │ │                     │ ╰───┴─────────╯                 │ │
            │                  │ │ wasi                │ [list 0 items]                  │ │
            │                  │ │                     │ ╭───┬─────────────╮             │ │
            │                  │ │ static-link-openssl │ │ 0 │ dep:openssl │             │ │
            │                  │ │                     │ ╰───┴─────────────╯             │ │
            │                  │ │                     │ ╭───┬─────────────────────────╮ │ │
            │                  │ │ which-support       │ │ 0 │ nu-command/which-suppor │ │ │
            │                  │ │                     │ │   │ t                       │ │ │
            │                  │ │                     │ ╰───┴─────────────────────────╯ │ │
            │                  │ │                     │ ╭───┬─────────────────────────╮ │ │
            │                  │ │ trash-support       │ │ 0 │ nu-command/trash-suppor │ │ │
            │                  │ │                     │ │   │ t                       │ │ │
            │                  │ │                     │ ╰───┴─────────────────────────╯ │ │
            │                  │ ╰─────────────────────┴─────────────────────────────────╯ │
            │                  │ ╭───┬──────┬─────────────╮                                │
            │ bin              │ │ # │ name │    path     │                                │
            │                  │ ├───┼──────┼─────────────┤                                │
            │                  │ │ 0 │ nu   │ src/main.rs │                                │
            │                  │ ╰───┴──────┴─────────────╯                                │
            │                  │ ╭───────────┬───────────────────────────────────────────╮ │
            │ patch            │ │           │ ╭──────────┬────────────────────────────╮ │ │
            │                  │ │ crates-io │ │          │ ╭────────┬───────────────╮ │ │ │
            │                  │ │           │ │ reedline │ │ git    │ https://githu │ │ │ │
            │                  │ │           │ │          │ │        │ b.com/nushell │ │ │ │
            │                  │ │           │ │          │ │        │ /reedline.git │ │ │ │
            │                  │ │           │ │          │ │ branch │ main          │ │ │ │
            │                  │ │           │ │          │ ╰────────┴───────────────╯ │ │ │
            │                  │ │           │ ╰──────────┴────────────────────────────╯ │ │
            │                  │ ╰───────────┴───────────────────────────────────────────╯ │
            │                  │ ╭───┬────────────┬─────────╮                              │
            │ bench            │ │ # │    name    │ harness │                              │
            │                  │ ├───┼────────────┼─────────┤                              │
            │                  │ │ 0 │ benchmarks │ false   │                              │
            │                  │ ╰───┴────────────┴─────────╯                              │
            ╰──────────────────┴───────────────────────────────────────────────────────────╯"#};

        assert_eq!(actual, expected);

        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --expand --width=120")?;

        let expected = indoc! {r#"
            ╭──────────────────┬───────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                  │ ╭───────────────┬───────────────────────────────────────────────────────────────────────────────╮ │
            │ package          │ │               │ ╭───┬────────────────────────────────╮                                        │ │
            │                  │ │ authors       │ │ 0 │ The Nushell Project Developers │                                        │ │
            │                  │ │               │ ╰───┴────────────────────────────────╯                                        │ │
            │                  │ │ default-run   │ nu                                                                            │ │
            │                  │ │ description   │ A new type of shell                                                           │ │
            │                  │ │ documentation │ https://www.nushell.sh/book/                                                  │ │
            │                  │ │ edition       │ 2024                                                                          │ │
            │                  │ │               │ ╭───┬────────╮                                                                │ │
            │                  │ │ exclude       │ │ 0 │ images │                                                                │ │
            │                  │ │               │ ╰───┴────────╯                                                                │ │
            │                  │ │ homepage      │ https://www.nushell.sh                                                        │ │
            │                  │ │ license       │ MIT                                                                           │ │
            │                  │ │ name          │ nu                                                                            │ │
            │                  │ │ repository    │ https://github.com/nushell/nushell                                            │ │
            │                  │ │ rust-version  │ 1.60                                                                          │ │
            │                  │ │ version       │ 0.74.1                                                                        │ │
            │                  │ │               │ ╭──────────┬────────────────────────────────────────────────────────────────╮ │ │
            │                  │ │ metadata      │ │          │ ╭───────────┬────────────────────────────────────────────────╮ │ │ │
            │                  │ │               │ │ binstall │ │ pkg-url   │ { repo }/releases/download/{ version }/{ name  │ │ │ │
            │                  │ │               │ │          │ │           │ }-{ version }-{ target }.{ archive-format }    │ │ │ │
            │                  │ │               │ │          │ │ pkg-fmt   │ tgz                                            │ │ │ │
            │                  │ │               │ │          │ │           │ ╭────────────────────────┬───────────────────╮ │ │ │ │
            │                  │ │               │ │          │ │ overrides │ │                        │ ╭─────────┬─────╮ │ │ │ │ │
            │                  │ │               │ │          │ │           │ │ x86_64-pc-windows-msvc │ │ pkg-fmt │ zip │ │ │ │ │ │
            │                  │ │               │ │          │ │           │ │                        │ ╰─────────┴─────╯ │ │ │ │ │
            │                  │ │               │ │          │ │           │ ╰────────────────────────┴───────────────────╯ │ │ │ │
            │                  │ │               │ │          │ ╰───────────┴────────────────────────────────────────────────╯ │ │ │
            │                  │ │               │ ╰──────────┴────────────────────────────────────────────────────────────────╯ │ │
            │                  │ ╰───────────────┴───────────────────────────────────────────────────────────────────────────────╯ │
            │                  │ ╭─────────┬─────────────────────────────────────────╮                                             │
            │ workspace        │ │         │ ╭────┬────────────────────────────────╮ │                                             │
            │                  │ │ members │ │  0 │ crates/nu-cli                  │ │                                             │
            │                  │ │         │ │  1 │ crates/nu-engine               │ │                                             │
            │                  │ │         │ │  2 │ crates/nu-parser               │ │                                             │
            │                  │ │         │ │  3 │ crates/nu-system               │ │                                             │
            │                  │ │         │ │  4 │ crates/nu-command              │ │                                             │
            │                  │ │         │ │  5 │ crates/nu-protocol             │ │                                             │
            │                  │ │         │ │  6 │ crates/nu-plugin               │ │                                             │
            │                  │ │         │ │  7 │ crates/nu_plugin_inc           │ │                                             │
            │                  │ │         │ │  8 │ crates/nu_plugin_gstat         │ │                                             │
            │                  │ │         │ │  9 │ crates/nu_plugin_example       │ │                                             │
            │                  │ │         │ │ 10 │ crates/nu_plugin_query         │ │                                             │
            │                  │ │         │ │ 11 │ crates/nu_plugin_custom_values │ │                                             │
            │                  │ │         │ │ 12 │ crates/nu-utils                │ │                                             │
            │                  │ │         │ ╰────┴────────────────────────────────╯ │                                             │
            │                  │ ╰─────────┴─────────────────────────────────────────╯                                             │
            │                  │ ╭───────────────┬───────────────────────────────────────────╮                                     │
            │ dependencies     │ │               │ ╭──────────┬───────────────╮              │                                     │
            │                  │ │ chrono        │ │ version  │ 0.4.23        │              │                                     │
            │                  │ │               │ │          │ ╭───┬───────╮ │              │                                     │
            │                  │ │               │ │ features │ │ 0 │ serde │ │              │                                     │
            │                  │ │               │ │          │ ╰───┴───────╯ │              │                                     │
            │                  │ │               │ ╰──────────┴───────────────╯              │                                     │
            │                  │ │ crossterm     │ 0.24.0                                    │                                     │
            │                  │ │ ctrlc         │ 3.2.1                                     │                                     │
            │                  │ │ log           │ 0.4                                       │                                     │
            │                  │ │               │ ╭──────────┬────────────────────────────╮ │                                     │
            │                  │ │ miette        │ │ version  │ 5.5.0                      │ │                                     │
            │                  │ │               │ │          │ ╭───┬────────────────────╮ │ │                                     │
            │                  │ │               │ │ features │ │ 0 │ fancy-no-backtrace │ │ │                                     │
            │                  │ │               │ │          │ ╰───┴────────────────────╯ │ │                                     │
            │                  │ │               │ ╰──────────┴────────────────────────────╯ │                                     │
            │                  │ │ nu-ansi-term  │ 0.46.0                                    │                                     │
            │                  │ │               │ ╭─────────┬─────────────────╮             │                                     │
            │                  │ │ nu-cli        │ │ path    │ ./crates/nu-cli │             │                                     │
            │                  │ │               │ │ version │ 0.74.1          │             │                                     │
            │                  │ │               │ ╰─────────┴─────────────────╯             │                                     │
            │                  │ │               │ ╭─────────┬────────────────────╮          │                                     │
            │                  │ │ nu-engine     │ │ path    │ ./crates/nu-engine │          │                                     │
            │                  │ │               │ │ version │ 0.74.1             │          │                                     │
            │                  │ │               │ ╰─────────┴────────────────────╯          │                                     │
            │                  │ │               │ ╭──────────┬──────────────────╮           │                                     │
            │                  │ │ reedline      │ │ version  │ 0.14.0           │           │                                     │
            │                  │ │               │ │          │ ╭───┬──────────╮ │           │                                     │
            │                  │ │               │ │ features │ │ 0 │ bashisms │ │           │                                     │
            │                  │ │               │ │          │ │ 1 │ sqlite   │ │           │                                     │
            │                  │ │               │ │          │ ╰───┴──────────╯ │           │                                     │
            │                  │ │               │ ╰──────────┴──────────────────╯           │                                     │
            │                  │ │ rayon         │ 1.6.1                                     │                                     │
            │                  │ │ is_executable │ 1.0.1                                     │                                     │
            │                  │ │ simplelog     │ 0.12.0                                    │                                     │
            │                  │ │ time          │ 0.3.12                                    │                                     │
            │                  │ ╰───────────────┴───────────────────────────────────────────╯                                     │
            │                  │ ╭─────────────────────────────────┬─────────────────────────────────────────────────────────────╮ │
            │ target           │ │                                 │ ╭──────────────┬──────────────────────────────────────────╮ │ │
            │                  │ │ cfg(not(target_os = "windows")) │ │              │ ╭─────────────┬────────────────────────╮ │ │ │
            │                  │ │                                 │ │ dependencies │ │             │ ╭──────────┬─────────╮ │ │ │ │
            │                  │ │                                 │ │              │ │ openssl     │ │ version  │ 0.10.38 │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ │ features │ [list 1 │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ │          │  item]  │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ │ optional │ true    │ │ │ │ │
            │                  │ │                                 │ │              │ │             │ ╰──────────┴─────────╯ │ │ │ │
            │                  │ │                                 │ │              │ │ signal-hook │ {record 2 fields}      │ │ │ │
            │                  │ │                                 │ │              │ ╰─────────────┴────────────────────────╯ │ │ │
            │                  │ │                                 │ ╰──────────────┴──────────────────────────────────────────╯ │ │
            │                  │ │                                 │ ╭────────────────────┬──────────────────╮                   │ │
            │                  │ │ cfg(windows)                    │ │                    │ ╭────────┬─────╮ │                   │ │
            │                  │ │                                 │ │ build-dependencies │ │ winres │ 0.1 │ │                   │ │
            │                  │ │                                 │ │                    │ ╰────────┴─────╯ │                   │ │
            │                  │ │                                 │ ╰────────────────────┴──────────────────╯                   │ │
            │                  │ │                                 │ ╭──────────────┬──────────────────────────────────────────╮ │ │
            │                  │ │ cfg(target_family = "unix")     │ │              │ ╭──────┬───────────────────────────────╮ │ │ │
            │                  │ │                                 │ │ dependencies │ │      │ ╭──────────────────┬────────╮ │ │ │ │
            │                  │ │                                 │ │              │ │ nix  │ │ version          │ 0.25   │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │ default-features │ false  │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │ features         │ [list  │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │                  │ 4      │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ │                  │ items] │ │ │ │ │
            │                  │ │                                 │ │              │ │      │ ╰──────────────────┴────────╯ │ │ │ │
            │                  │ │                                 │ │              │ │ atty │ 0.2                           │ │ │ │
            │                  │ │                                 │ │              │ ╰──────┴───────────────────────────────╯ │ │ │
            │                  │ │                                 │ ╰──────────────┴──────────────────────────────────────────╯ │ │
            │                  │ ╰─────────────────────────────────┴─────────────────────────────────────────────────────────────╯ │
            │                  │ ╭───────────────────┬────────────────────────────────────────╮                                    │
            │ dev-dependencies │ │                   │ ╭─────────┬──────────────────────────╮ │                                    │
            │                  │ │ nu-test-support   │ │ path    │ ./crates/nu-test-support │ │                                    │
            │                  │ │                   │ │ version │ 0.74.1                   │ │                                    │
            │                  │ │                   │ ╰─────────┴──────────────────────────╯ │                                    │
            │                  │ │ tempfile          │ 3.2.0                                  │                                    │
            │                  │ │ assert_cmd        │ 2.0.2                                  │                                    │
            │                  │ │ criterion         │ 0.4                                    │                                    │
            │                  │ │ pretty_assertions │ 1.0.0                                  │                                    │
            │                  │ │ serial_test       │ 0.10.0                                 │                                    │
            │                  │ │ hamcrest2         │ 0.3.0                                  │                                    │
            │                  │ │                   │ ╭──────────────────┬────────╮          │                                    │
            │                  │ │ rstest            │ │ version          │ 0.15.0 │          │                                    │
            │                  │ │                   │ │ default-features │ false  │          │                                    │
            │                  │ │                   │ ╰──────────────────┴────────╯          │                                    │
            │                  │ │ itertools         │ 0.10.3                                 │                                    │
            │                  │ ╰───────────────────┴────────────────────────────────────────╯                                    │
            │                  │ ╭─────────────────────┬──────────────────────────────────╮                                        │
            │ features         │ │                     │ ╭───┬────────────────────╮       │                                        │
            │                  │ │ plugin              │ │ 0 │ nu-plugin          │       │                                        │
            │                  │ │                     │ │ 1 │ nu-cli/plugin      │       │                                        │
            │                  │ │                     │ │ 2 │ nu-parser/plugin   │       │                                        │
            │                  │ │                     │ │ 3 │ nu-command/plugin  │       │                                        │
            │                  │ │                     │ │ 4 │ nu-protocol/plugin │       │                                        │
            │                  │ │                     │ │ 5 │ nu-engine/plugin   │       │                                        │
            │                  │ │                     │ ╰───┴────────────────────╯       │                                        │
            │                  │ │                     │ ╭───┬─────────╮                  │                                        │
            │                  │ │ extra               │ │ 0 │ default │                  │                                        │
            │                  │ │                     │ ╰───┴─────────╯                  │                                        │
            │                  │ │                     │ ╭───┬───────────────╮            │                                        │
            │                  │ │ default             │ │ 0 │ plugin        │            │                                        │
            │                  │ │                     │ │ 1 │ which-support │            │                                        │
            │                  │ │                     │ │ 2 │ trash-support │            │                                        │
            │                  │ │                     │ │ 3 │ sqlite        │            │                                        │
            │                  │ │                     │ ╰───┴───────────────╯            │                                        │
            │                  │ │                     │ ╭───┬─────────╮                  │                                        │
            │                  │ │ stable              │ │ 0 │ default │                  │                                        │
            │                  │ │                     │ ╰───┴─────────╯                  │                                        │
            │                  │ │ wasi                │ [list 0 items]                   │                                        │
            │                  │ │                     │ ╭───┬─────────────╮              │                                        │
            │                  │ │ static-link-openssl │ │ 0 │ dep:openssl │              │                                        │
            │                  │ │                     │ ╰───┴─────────────╯              │                                        │
            │                  │ │                     │ ╭───┬──────────────────────────╮ │                                        │
            │                  │ │ which-support       │ │ 0 │ nu-command/which-support │ │                                        │
            │                  │ │                     │ ╰───┴──────────────────────────╯ │                                        │
            │                  │ │                     │ ╭───┬──────────────────────────╮ │                                        │
            │                  │ │ trash-support       │ │ 0 │ nu-command/trash-support │ │                                        │
            │                  │ │                     │ ╰───┴──────────────────────────╯ │                                        │
            │                  │ ╰─────────────────────┴──────────────────────────────────╯                                        │
            │                  │ ╭───┬──────┬─────────────╮                                                                        │
            │ bin              │ │ # │ name │    path     │                                                                        │
            │                  │ ├───┼──────┼─────────────┤                                                                        │
            │                  │ │ 0 │ nu   │ src/main.rs │                                                                        │
            │                  │ ╰───┴──────┴─────────────╯                                                                        │
            │                  │ ╭───────────┬───────────────────────────────────────────────────────────────────────────────────╮ │
            │ patch            │ │           │ ╭─────────────────┬─────────────────────────────────────────────────────────────╮ │ │
            │                  │ │ crates-io │ │                 │ ╭────────┬─────────────────────────────────────────╮        │ │ │
            │                  │ │           │ │ reedline        │ │ git    │ https://github.com/nushell/reedline.git │        │ │ │
            │                  │ │           │ │                 │ │ branch │ main                                    │        │ │ │
            │                  │ │           │ │                 │ ╰────────┴─────────────────────────────────────────╯        │ │ │
            │                  │ │           │ ╰─────────────────┴─────────────────────────────────────────────────────────────╯ │ │
            │                  │ ╰───────────┴───────────────────────────────────────────────────────────────────────────────────╯ │
            │                  │ ╭───┬────────────┬─────────╮                                                                      │
            │ bench            │ │ # │    name    │ harness │                                                                      │
            │                  │ ├───┼────────────┼─────────┤                                                                      │
            │                  │ │ 0 │ benchmarks │ false   │                                                                      │
            │                  │ ╰───┴────────────┴─────────╯                                                                      │
            ╰──────────────────┴───────────────────────────────────────────────────────────────────────────────────────────────────╯"#};

        assert_eq!(actual, expected);

        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --expand --width=60")?;

        let expected = indoc! {r#"
            ╭──────────────────┬───────────────────────────────────────╮
            │                  │ ╭───────────────┬───────────────────╮ │
            │ package          │ │               │ ╭───┬───────────╮ │ │
            │                  │ │ authors       │ │ 0 │ The       │ │ │
            │                  │ │               │ │   │ Nushell   │ │ │
            │                  │ │               │ │   │ Project D │ │ │
            │                  │ │               │ │   │ evelopers │ │ │
            │                  │ │               │ ╰───┴───────────╯ │ │
            │                  │ │ default-run   │ nu                │ │
            │                  │ │ description   │ A new type of     │ │
            │                  │ │               │ shell             │ │
            │                  │ │ documentation │ https://www.nushe │ │
            │                  │ │               │ ll.sh/book/       │ │
            │                  │ │ edition       │ 2024              │ │
            │                  │ │               │ ╭───┬────────╮    │ │
            │                  │ │ exclude       │ │ 0 │ images │    │ │
            │                  │ │               │ ╰───┴────────╯    │ │
            │                  │ │ homepage      │ https://www.nushe │ │
            │                  │ │               │ ll.sh             │ │
            │                  │ │ license       │ MIT               │ │
            │                  │ │ name          │ nu                │ │
            │                  │ │ repository    │ https://github.co │ │
            │                  │ │               │ m/nushell/nushell │ │
            │                  │ │ rust-version  │ 1.60              │ │
            │                  │ │ version       │ 0.74.1            │ │
            │                  │ │               │ ╭──────────┬────╮ │ │
            │                  │ │ metadata      │ │ binstall │ {r │ │ │
            │                  │ │               │ │          │ ec │ │ │
            │                  │ │               │ │          │ or │ │ │
            │                  │ │               │ │          │ d  │ │ │
            │                  │ │               │ │          │ 3  │ │ │
            │                  │ │               │ │          │ fi │ │ │
            │                  │ │               │ │          │ el │ │ │
            │                  │ │               │ │          │ ds │ │ │
            │                  │ │               │ │          │ }  │ │ │
            │                  │ │               │ ╰──────────┴────╯ │ │
            │                  │ ╰───────────────┴───────────────────╯ │
            │                  │ ╭─────────┬─────────────────────────╮ │
            │ workspace        │ │         │ ╭────┬────────────────╮ │ │
            │                  │ │ members │ │  0 │ crates/nu-cli  │ │ │
            │                  │ │         │ │  1 │ crates/nu-engi │ │ │
            │                  │ │         │ │    │ ne             │ │ │
            │                  │ │         │ │  2 │ crates/nu-pars │ │ │
            │                  │ │         │ │    │ er             │ │ │
            │                  │ │         │ │  3 │ crates/nu-syst │ │ │
            │                  │ │         │ │    │ em             │ │ │
            │                  │ │         │ │  4 │ crates/nu-comm │ │ │
            │                  │ │         │ │    │ and            │ │ │
            │                  │ │         │ │  5 │ crates/nu-prot │ │ │
            │                  │ │         │ │    │ ocol           │ │ │
            │                  │ │         │ │  6 │ crates/nu-plug │ │ │
            │                  │ │         │ │    │ in             │ │ │
            │                  │ │         │ │  7 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_inc         │ │ │
            │                  │ │         │ │  8 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_gstat       │ │ │
            │                  │ │         │ │  9 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_example     │ │ │
            │                  │ │         │ │ 10 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_query       │ │ │
            │                  │ │         │ │ 11 │ crates/nu_plug │ │ │
            │                  │ │         │ │    │ in_custom_valu │ │ │
            │                  │ │         │ │    │ es             │ │ │
            │                  │ │         │ │ 12 │ crates/nu-util │ │ │
            │                  │ │         │ │    │ s              │ │ │
            │                  │ │         │ ╰────┴────────────────╯ │ │
            │                  │ ╰─────────┴─────────────────────────╯ │
            │                  │ ╭───────────────┬───────────────────╮ │
            │ dependencies     │ │               │ ╭──────────┬────╮ │ │
            │                  │ │ chrono        │ │ version  │ 0. │ │ │
            │                  │ │               │ │          │ 4. │ │ │
            │                  │ │               │ │          │ 23 │ │ │
            │                  │ │               │ │ features │ [l │ │ │
            │                  │ │               │ │          │ is │ │ │
            │                  │ │               │ │          │ t  │ │ │
            │                  │ │               │ │          │ 1  │ │ │
            │                  │ │               │ │          │ it │ │ │
            │                  │ │               │ │          │ em │ │ │
            │                  │ │               │ │          │ ]  │ │ │
            │                  │ │               │ ╰──────────┴────╯ │ │
            │                  │ │ crossterm     │ 0.24.0            │ │
            │                  │ │ ctrlc         │ 3.2.1             │ │
            │                  │ │ log           │ 0.4               │ │
            │                  │ │               │ ╭──────────┬────╮ │ │
            │                  │ │ miette        │ │ version  │ 5. │ │ │
            │                  │ │               │ │          │ 5. │ │ │
            │                  │ │               │ │          │ 0  │ │ │
            │                  │ │               │ │ features │ [l │ │ │
            │                  │ │               │ │          │ is │ │ │
            │                  │ │               │ │          │ t  │ │ │
            │                  │ │               │ │          │ 1  │ │ │
            │                  │ │               │ │          │ it │ │ │
            │                  │ │               │ │          │ em │ │ │
            │                  │ │               │ │          │ ]  │ │ │
            │                  │ │               │ ╰──────────┴────╯ │ │
            │                  │ │ nu-ansi-term  │ 0.46.0            │ │
            │                  │ │               │ ╭─────────┬─────╮ │ │
            │                  │ │ nu-cli        │ │ path    │ ./c │ │ │
            │                  │ │               │ │         │ rat │ │ │
            │                  │ │               │ │         │ es/ │ │ │
            │                  │ │               │ │         │ nu- │ │ │
            │                  │ │               │ │         │ cli │ │ │
            │                  │ │               │ │ version │ 0.7 │ │ │
            │                  │ │               │ │         │ 4.1 │ │ │
            │                  │ │               │ ╰─────────┴─────╯ │ │
            │                  │ │               │ ╭─────────┬─────╮ │ │
            │                  │ │ nu-engine     │ │ path    │ ./c │ │ │
            │                  │ │               │ │         │ rat │ │ │
            │                  │ │               │ │         │ es/ │ │ │
            │                  │ │               │ │         │ nu- │ │ │
            │                  │ │               │ │         │ eng │ │ │
            │                  │ │               │ │         │ ine │ │ │
            │                  │ │               │ │ version │ 0.7 │ │ │
            │                  │ │               │ │         │ 4.1 │ │ │
            │                  │ │               │ ╰─────────┴─────╯ │ │
            │                  │ │               │ ╭──────────┬────╮ │ │
            │                  │ │ reedline      │ │ version  │ 0. │ │ │
            │                  │ │               │ │          │ 14 │ │ │
            │                  │ │               │ │          │ .0 │ │ │
            │                  │ │               │ │ features │ [l │ │ │
            │                  │ │               │ │          │ is │ │ │
            │                  │ │               │ │          │ t  │ │ │
            │                  │ │               │ │          │ 2  │ │ │
            │                  │ │               │ │          │ it │ │ │
            │                  │ │               │ │          │ em │ │ │
            │                  │ │               │ │          │ s] │ │ │
            │                  │ │               │ ╰──────────┴────╯ │ │
            │                  │ │ rayon         │ 1.6.1             │ │
            │                  │ │ is_executable │ 1.0.1             │ │
            │                  │ │ simplelog     │ 0.12.0            │ │
            │                  │ │ time          │ 0.3.12            │ │
            │                  │ ╰───────────────┴───────────────────╯ │
            │ target           │ {record 3 fields}                     │
            │                  │ ╭─────────────────────┬─────────────╮ │
            │ dev-dependencies │ │ nu-test-support     │ {record 2   │ │
            │                  │ │                     │ fields}     │ │
            │                  │ │ tempfile            │ 3.2.0       │ │
            │                  │ │ assert_cmd          │ 2.0.2       │ │
            │                  │ │ criterion           │ 0.4         │ │
            │                  │ │ pretty_assertions   │ 1.0.0       │ │
            │                  │ │ serial_test         │ 0.10.0      │ │
            │                  │ │ hamcrest2           │ 0.3.0       │ │
            │                  │ │ rstest              │ {record 2   │ │
            │                  │ │                     │ fields}     │ │
            │                  │ │ itertools           │ 0.10.3      │ │
            │                  │ ╰─────────────────────┴─────────────╯ │
            │                  │ ╭─────────────────────┬─────────────╮ │
            │ features         │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ plugin              │ │ 0 │ nu- │ │ │
            │                  │ │                     │ │   │ plu │ │ │
            │                  │ │                     │ │   │ gin │ │ │
            │                  │ │                     │ │ 1 │ nu- │ │ │
            │                  │ │                     │ │   │ cli │ │ │
            │                  │ │                     │ │   │ /pl │ │ │
            │                  │ │                     │ │   │ ugi │ │ │
            │                  │ │                     │ │   │ n   │ │ │
            │                  │ │                     │ │ 2 │ nu- │ │ │
            │                  │ │                     │ │   │ par │ │ │
            │                  │ │                     │ │   │ ser │ │ │
            │                  │ │                     │ │   │ /pl │ │ │
            │                  │ │                     │ │   │ ugi │ │ │
            │                  │ │                     │ │   │ n   │ │ │
            │                  │ │                     │ │ 3 │ nu- │ │ │
            │                  │ │                     │ │   │ com │ │ │
            │                  │ │                     │ │   │ man │ │ │
            │                  │ │                     │ │   │ d/p │ │ │
            │                  │ │                     │ │   │ lug │ │ │
            │                  │ │                     │ │   │ in  │ │ │
            │                  │ │                     │ │ 4 │ nu- │ │ │
            │                  │ │                     │ │   │ pro │ │ │
            │                  │ │                     │ │   │ toc │ │ │
            │                  │ │                     │ │   │ ol/ │ │ │
            │                  │ │                     │ │   │ plu │ │ │
            │                  │ │                     │ │   │ gin │ │ │
            │                  │ │                     │ │ 5 │ nu- │ │ │
            │                  │ │                     │ │   │ eng │ │ │
            │                  │ │                     │ │   │ ine │ │ │
            │                  │ │                     │ │   │ /pl │ │ │
            │                  │ │                     │ │   │ ugi │ │ │
            │                  │ │                     │ │   │ n   │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ extra               │ │ 0 │ def │ │ │
            │                  │ │                     │ │   │ aul │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ default             │ │ 0 │ plu │ │ │
            │                  │ │                     │ │   │ gin │ │ │
            │                  │ │                     │ │ 1 │ whi │ │ │
            │                  │ │                     │ │   │ ch- │ │ │
            │                  │ │                     │ │   │ sup │ │ │
            │                  │ │                     │ │   │ por │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ │ 2 │ tra │ │ │
            │                  │ │                     │ │   │ sh- │ │ │
            │                  │ │                     │ │   │ sup │ │ │
            │                  │ │                     │ │   │ por │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ │ 3 │ sql │ │ │
            │                  │ │                     │ │   │ ite │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ stable              │ │ 0 │ def │ │ │
            │                  │ │                     │ │   │ aul │ │ │
            │                  │ │                     │ │   │ t   │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │ wasi                │ [list 0     │ │
            │                  │ │                     │ items]      │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ static-link-openssl │ │ 0 │ dep │ │ │
            │                  │ │                     │ │   │ :op │ │ │
            │                  │ │                     │ │   │ ens │ │ │
            │                  │ │                     │ │   │ sl  │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ which-support       │ │ 0 │ nu- │ │ │
            │                  │ │                     │ │   │ com │ │ │
            │                  │ │                     │ │   │ man │ │ │
            │                  │ │                     │ │   │ d/w │ │ │
            │                  │ │                     │ │   │ hic │ │ │
            │                  │ │                     │ │   │ h-s │ │ │
            │                  │ │                     │ │   │ upp │ │ │
            │                  │ │                     │ │   │ ort │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ │                     │ ╭───┬─────╮ │ │
            │                  │ │ trash-support       │ │ 0 │ nu- │ │ │
            │                  │ │                     │ │   │ com │ │ │
            │                  │ │                     │ │   │ man │ │ │
            │                  │ │                     │ │   │ d/t │ │ │
            │                  │ │                     │ │   │ ras │ │ │
            │                  │ │                     │ │   │ h-s │ │ │
            │                  │ │                     │ │   │ upp │ │ │
            │                  │ │                     │ │   │ ort │ │ │
            │                  │ │                     │ ╰───┴─────╯ │ │
            │                  │ ╰─────────────────────┴─────────────╯ │
            │                  │ ╭───┬──────┬─────────────╮            │
            │ bin              │ │ # │ name │    path     │            │
            │                  │ ├───┼──────┼─────────────┤            │
            │                  │ │ 0 │ nu   │ src/main.rs │            │
            │                  │ ╰───┴──────┴─────────────╯            │
            │                  │ ╭───────────┬───────────────────────╮ │
            │ patch            │ │           │ ╭──────────┬────────╮ │ │
            │                  │ │ crates-io │ │ reedline │ {recor │ │ │
            │                  │ │           │ │          │ d 2 fi │ │ │
            │                  │ │           │ │          │ elds}  │ │ │
            │                  │ │           │ ╰──────────┴────────╯ │ │
            │                  │ ╰───────────┴───────────────────────╯ │
            │                  │ ╭───┬────────────┬─────────╮          │
            │ bench            │ │ # │    name    │ harness │          │
            │                  │ ├───┼────────────┼─────────┤          │
            │                  │ │ 0 │ benchmarks │ false   │          │
            │                  │ ╰───┴────────────┴─────────╯          │
            ╰──────────────────┴───────────────────────────────────────╯"#};

        assert_eq!(actual, expected);

        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --expand --width=40")?;

        let expected = indoc! {r#"
            ╭──────────────────┬───────────────────╮
            │ package          │ {record 13        │
            │                  │ fields}           │
            │                  │ ╭─────────┬─────╮ │
            │ workspace        │ │ members │ [li │ │
            │                  │ │         │ st  │ │
            │                  │ │         │ 13  │ │
            │                  │ │         │ ite │ │
            │                  │ │         │ ms] │ │
            │                  │ ╰─────────┴─────╯ │
            │ dependencies     │ {record 13        │
            │                  │ fields}           │
            │ target           │ {record 3 fields} │
            │ dev-dependencies │ {record 9 fields} │
            │ features         │ {record 8 fields} │
            │                  │ ╭───┬──────┬────╮ │
            │ bin              │ │ # │ name │ pa │ │
            │                  │ │   │      │ th │ │
            │                  │ ├───┼──────┼────┤ │
            │                  │ │ 0 │ nu   │ sr │ │
            │                  │ │   │      │ c/ │ │
            │                  │ │   │      │ ma │ │
            │                  │ │   │      │ in │ │
            │                  │ │   │      │ .r │ │
            │                  │ │   │      │ s  │ │
            │                  │ ╰───┴──────┴────╯ │
            │                  │ ╭───────────┬───╮ │
            │ patch            │ │ crates-io │ { │ │
            │                  │ │           │ r │ │
            │                  │ │           │ e │ │
            │                  │ │           │ c │ │
            │                  │ │           │ o │ │
            │                  │ │           │ r │ │
            │                  │ │           │ d │ │
            │                  │ │           │   │ │
            │                  │ │           │ 1 │ │
            │                  │ │           │   │ │
            │                  │ │           │ f │ │
            │                  │ │           │ i │ │
            │                  │ │           │ e │ │
            │                  │ │           │ l │ │
            │                  │ │           │ d │ │
            │                  │ │           │ } │ │
            │                  │ ╰───────────┴───╯ │
            │                  │ ╭───┬───────┬───╮ │
            │ bench            │ │ # │ name  │ h │ │
            │                  │ │   │       │ a │ │
            │                  │ │   │       │ r │ │
            │                  │ │   │       │ n │ │
            │                  │ │   │       │ e │ │
            │                  │ │   │       │ s │ │
            │                  │ │   │       │ s │ │
            │                  │ ├───┼───────┼───┤ │
            │                  │ │ 0 │ bench │ f │ │
            │                  │ │   │ marks │ a │ │
            │                  │ │   │       │ l │ │
            │                  │ │   │       │ s │ │
            │                  │ │   │       │ e │ │
            │                  │ ╰───┴───────┴───╯ │
            ╰──────────────────┴───────────────────╯"#};

        assert_eq!(actual, expected);
        Ok(())
    })
}

#[test]
fn table_expande_with_no_header_internally_0() -> Result {
    let nu_value = r##"{ "config            ": { "ls": { "use_ls_colors": true, "clickable_links": false }, "rm": { "always_trash": false }, "cd": { "abbreviations": false }, "table": { "mode": "rounded", "index_mode": "always", "trim": { "methodology": "wrapping", "wrapping_try_keep_words": true, "truncating_suffix": "..." } }, "explore": { "help_banner": true, "exit_esc": true, "command_bar_text": "#C4C9C6", "status_bar_background": { "fg": "#1D1F21", "bg": "#C4C9C6" }, "highlight": { "bg": "yellow", "fg": "black" }, "status": {}, "try": {}, "table": { "split_line": "#404040", "cursor": true, "line_index": true, "line_shift": true, "line_head_top": true, "line_head_bottom": true, "show_head": true, "show_index": true }, "config": { "cursor_color": { "bg": "yellow", "fg": "black" } } }, "history": { "max_size": 10000, "sync_on_enter": true, "file_format": "plaintext" }, "completions": { "case_sensitive": false, "quick": true, "partial": true, "algorithm": "prefix", "external": { "enable": true, "max_results": 100, "completer": null } }, "filesize": { "metric": true, "format": "auto" }, "cursor_shape": { "emacs": "line", "vi_insert": "block", "vi_normal": "underscore" }, "color_config": { "separator": "default", "leading_trailing_space_bg": { "attr": "n" }, "header": "green_bold", "empty": "blue", "bool": null, "int": "default", "filesize": null, "duration": "default", "datetime": null, "range": "default", "float": "default", "string": "default", "nothing": "default", "binary": "default", "cell-path": "default", "row_index": "green_bold", "record": "default", "list": "default", "block": "default", "hints": "dark_gray", "search_result": {"fg": "white", "bg": "red"}, "shape_and": "purple_bold", "shape_binary": "purple_bold", "shape_block": "blue_bold", "shape_bool": "light_cyan", "shape_custom": "green", "shape_datetime": "cyan_bold", "shape_directory": "cyan", "shape_external": "cyan", "shape_externalarg": "green_bold", "shape_filepath": "cyan", "shape_flag": "blue_bold", "shape_float": "purple_bold", "shape_garbage": { "fg": "#FFFFFF", "bg": "#FF0000", "attr": "b" }, "shape_globpattern": "cyan_bold", "shape_int": "purple_bold", "shape_internalcall": "cyan_bold", "shape_list": "cyan_bold", "shape_literal": "blue", "shape_matching_brackets": { "attr": "u" }, "shape_nothing": "light_cyan", "shape_operator": "yellow", "shape_or": "purple_bold", "shape_pipe": "purple_bold", "shape_range": "yellow_bold", "shape_record": "cyan_bold", "shape_redirection": "purple_bold", "shape_signature": "green_bold", "shape_string": "green", "shape_string_interpolation": "cyan_bold", "shape_table": "blue_bold", "shape_variable": "purple" }, "footer_mode": "25", "float_precision": 2, "use_ansi_coloring": true, "edit_mode": "emacs", "shell_integration": true, "show_banner": true, "render_right_prompt_on_last_line": false, "hooks": { "pre_prompt": [ null ], "pre_execution": [ null ], "env_change": { "PWD": [ null ] }, "display_output": null }, "menus": [ { "name": "completion_menu", "only_buffer_difference": false, "marker": "| ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "history_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "help_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "commands_menu", "only_buffer_difference": false, "marker": "# ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "vars_menu", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "commands_with_description", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null } ], "keybindings": [ { "name": "completion_menu", "modifier": "none", "keycode": "tab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "send": "menu", "name": "completion_menu" }, { "send": "menunext" } ] } }, { "name": "completion_previous", "modifier": "shift", "keycode": "backtab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menuprevious" } }, { "name": "history_menu", "modifier": "control", "keycode": "char_r", "mode": "emacs", "event": { "send": "menu", "name": "history_menu" } }, { "name": "next_page", "modifier": "control", "keycode": "char_x", "mode": "emacs", "event": { "send": "menupagenext" } }, { "name": "undo_or_previous_page", "modifier": "control", "keycode": "char_z", "mode": "emacs", "event": { "until": [ { "send": "menupageprevious" }, { "edit": "undo" } ] } }, { "name": "yank", "modifier": "control", "keycode": "char_y", "mode": "emacs", "event": { "until": [ { "edit": "pastecutbufferafter" } ] } }, { "name": "unix-line-discard", "modifier": "control", "keycode": "char_u", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cutfromlinestart" } ] } }, { "name": "kill-line", "modifier": "control", "keycode": "char_k", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cuttolineend" } ] } }, { "name": "commands_menu", "modifier": "control", "keycode": "char_t", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_menu" } }, { "name": "vars_menu", "modifier": "alt", "keycode": "char_o", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "vars_menu" } }, { "name": "commands_with_description", "modifier": "control", "keycode": "char_s", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_with_description" } } ] } }"##;

    let mut tester = test();
    let data: Value = tester.run(nu_value.trim())?;
    let actual: String = tester.run_with_data("table --expand --width 141", data)?;

    assert_eq!(
        actual,
        indoc! {r#"
            ╭────────────────────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                    │ ╭──────────────────────────────────┬───────────────────────────────────────────────────────────────────────────────╮ │
            │ config             │ │                                  │ ╭─────────────────┬───────╮                                                   │ │
            │                    │ │ ls                               │ │ use_ls_colors   │ true  │                                                   │ │
            │                    │ │                                  │ │ clickable_links │ false │                                                   │ │
            │                    │ │                                  │ ╰─────────────────┴───────╯                                                   │ │
            │                    │ │                                  │ ╭──────────────┬───────╮                                                      │ │
            │                    │ │ rm                               │ │ always_trash │ false │                                                      │ │
            │                    │ │                                  │ ╰──────────────┴───────╯                                                      │ │
            │                    │ │                                  │ ╭───────────────┬───────╮                                                     │ │
            │                    │ │ cd                               │ │ abbreviations │ false │                                                     │ │
            │                    │ │                                  │ ╰───────────────┴───────╯                                                     │ │
            │                    │ │                                  │ ╭────────────┬────────────────────────────────────────╮                       │ │
            │                    │ │ table                            │ │ mode       │ rounded                                │                       │ │
            │                    │ │                                  │ │ index_mode │ always                                 │                       │ │
            │                    │ │                                  │ │            │ ╭─────────────────────────┬──────────╮ │                       │ │
            │                    │ │                                  │ │ trim       │ │ methodology             │ wrapping │ │                       │ │
            │                    │ │                                  │ │            │ │ wrapping_try_keep_words │ true     │ │                       │ │
            │                    │ │                                  │ │            │ │ truncating_suffix       │ ...      │ │                       │ │
            │                    │ │                                  │ │            │ ╰─────────────────────────┴──────────╯ │                       │ │
            │                    │ │                                  │ ╰────────────┴────────────────────────────────────────╯                       │ │
            │                    │ │                                  │ ╭───────────────────────────────┬───────────────────────────────────────────╮ │ │
            │                    │ │ explore                          │ │ help_banner                   │ true                                      │ │ │
            │                    │ │                                  │ │ exit_esc                      │ true                                      │ │ │
            │                    │ │                                  │ │ command_bar_text              │ #C4C9C6                                   │ │ │
            │                    │ │                                  │ │                               │ ╭────┬─────────╮                          │ │ │
            │                    │ │                                  │ │ status_bar_background         │ │ fg │ #1D1F21 │                          │ │ │
            │                    │ │                                  │ │                               │ │ bg │ #C4C9C6 │                          │ │ │
            │                    │ │                                  │ │                               │ ╰────┴─────────╯                          │ │ │
            │                    │ │                                  │ │                               │ ╭────┬────────╮                           │ │ │
            │                    │ │                                  │ │ highlight                     │ │ bg │ yellow │                           │ │ │
            │                    │ │                                  │ │                               │ │ fg │ black  │                           │ │ │
            │                    │ │                                  │ │                               │ ╰────┴────────╯                           │ │ │
            │                    │ │                                  │ │ status                        │ {record 0 fields}                         │ │ │
            │                    │ │                                  │ │ try                           │ {record 0 fields}                         │ │ │
            │                    │ │                                  │ │                               │ ╭──────────────────┬─────────╮            │ │ │
            │                    │ │                                  │ │ table                         │ │ split_line       │ #404040 │            │ │ │
            │                    │ │                                  │ │                               │ │ cursor           │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_index       │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_shift       │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_head_top    │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ line_head_bottom │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ show_head        │ true    │            │ │ │
            │                    │ │                                  │ │                               │ │ show_index       │ true    │            │ │ │
            │                    │ │                                  │ │                               │ ╰──────────────────┴─────────╯            │ │ │
            │                    │ │                                  │ │                               │ ╭──────────────┬─────────────────╮        │ │ │
            │                    │ │                                  │ │ config                        │ │              │ ╭────┬────────╮ │        │ │ │
            │                    │ │                                  │ │                               │ │ cursor_color │ │ bg │ yellow │ │        │ │ │
            │                    │ │                                  │ │                               │ │              │ │ fg │ black  │ │        │ │ │
            │                    │ │                                  │ │                               │ │              │ ╰────┴────────╯ │        │ │ │
            │                    │ │                                  │ │                               │ ╰──────────────┴─────────────────╯        │ │ │
            │                    │ │                                  │ ╰───────────────────────────────┴───────────────────────────────────────────╯ │ │
            │                    │ │                                  │ ╭───────────────┬───────────╮                                                 │ │
            │                    │ │ history                          │ │ max_size      │ 10000     │                                                 │ │
            │                    │ │                                  │ │ sync_on_enter │ true      │                                                 │ │
            │                    │ │                                  │ │ file_format   │ plaintext │                                                 │ │
            │                    │ │                                  │ ╰───────────────┴───────────╯                                                 │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────────╮                                   │ │
            │                    │ │ completions                      │ │ case_sensitive │ false                  │                                   │ │
            │                    │ │                                  │ │ quick          │ true                   │                                   │ │
            │                    │ │                                  │ │ partial        │ true                   │                                   │ │
            │                    │ │                                  │ │ algorithm      │ prefix                 │                                   │ │
            │                    │ │                                  │ │                │ ╭─────────────┬──────╮ │                                   │ │
            │                    │ │                                  │ │ external       │ │ enable      │ true │ │                                   │ │
            │                    │ │                                  │ │                │ │ max_results │ 100  │ │                                   │ │
            │                    │ │                                  │ │                │ │ completer   │      │ │                                   │ │
            │                    │ │                                  │ │                │ ╰─────────────┴──────╯ │                                   │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────────╯                                   │ │
            │                    │ │                                  │ ╭────────┬──────╮                                                             │ │
            │                    │ │ filesize                         │ │ metric │ true │                                                             │ │
            │                    │ │                                  │ │ format │ auto │                                                             │ │
            │                    │ │                                  │ ╰────────┴──────╯                                                             │ │
            │                    │ │                                  │ ╭───────────┬────────────╮                                                    │ │
            │                    │ │ cursor_shape                     │ │ emacs     │ line       │                                                    │ │
            │                    │ │                                  │ │ vi_insert │ block      │                                                    │ │
            │                    │ │                                  │ │ vi_normal │ underscore │                                                    │ │
            │                    │ │                                  │ ╰───────────┴────────────╯                                                    │ │
            │                    │ │                                  │ ╭────────────────────────────┬────────────────────╮                           │ │
            │                    │ │ color_config                     │ │ separator                  │ default            │                           │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                           │ │
            │                    │ │                                  │ │ leading_trailing_space_bg  │ │ attr │ n │       │                           │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                           │ │
            │                    │ │                                  │ │ header                     │ green_bold         │                           │ │
            │                    │ │                                  │ │ empty                      │ blue               │                           │ │
            │                    │ │                                  │ │ bool                       │                    │                           │ │
            │                    │ │                                  │ │ int                        │ default            │                           │ │
            │                    │ │                                  │ │ filesize                   │                    │                           │ │
            │                    │ │                                  │ │ duration                   │ default            │                           │ │
            │                    │ │                                  │ │ datetime                   │                    │                           │ │
            │                    │ │                                  │ │ range                      │ default            │                           │ │
            │                    │ │                                  │ │ float                      │ default            │                           │ │
            │                    │ │                                  │ │ string                     │ default            │                           │ │
            │                    │ │                                  │ │ nothing                    │ default            │                           │ │
            │                    │ │                                  │ │ binary                     │ default            │                           │ │
            │                    │ │                                  │ │ cell-path                  │ default            │                           │ │
            │                    │ │                                  │ │ row_index                  │ green_bold         │                           │ │
            │                    │ │                                  │ │ record                     │ default            │                           │ │
            │                    │ │                                  │ │ list                       │ default            │                           │ │
            │                    │ │                                  │ │ block                      │ default            │                           │ │
            │                    │ │                                  │ │ hints                      │ dark_gray          │                           │ │
            │                    │ │                                  │ │                            │ ╭────┬───────╮     │                           │ │
            │                    │ │                                  │ │ search_result              │ │ fg │ white │     │                           │ │
            │                    │ │                                  │ │                            │ │ bg │ red   │     │                           │ │
            │                    │ │                                  │ │                            │ ╰────┴───────╯     │                           │ │
            │                    │ │                                  │ │ shape_and                  │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_binary               │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_block                │ blue_bold          │                           │ │
            │                    │ │                                  │ │ shape_bool                 │ light_cyan         │                           │ │
            │                    │ │                                  │ │ shape_custom               │ green              │                           │ │
            │                    │ │                                  │ │ shape_datetime             │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_directory            │ cyan               │                           │ │
            │                    │ │                                  │ │ shape_external             │ cyan               │                           │ │
            │                    │ │                                  │ │ shape_externalarg          │ green_bold         │                           │ │
            │                    │ │                                  │ │ shape_filepath             │ cyan               │                           │ │
            │                    │ │                                  │ │ shape_flag                 │ blue_bold          │                           │ │
            │                    │ │                                  │ │ shape_float                │ purple_bold        │                           │ │
            │                    │ │                                  │ │                            │ ╭──────┬─────────╮ │                           │ │
            │                    │ │                                  │ │ shape_garbage              │ │ fg   │ #FFFFFF │ │                           │ │
            │                    │ │                                  │ │                            │ │ bg   │ #FF0000 │ │                           │ │
            │                    │ │                                  │ │                            │ │ attr │ b       │ │                           │ │
            │                    │ │                                  │ │                            │ ╰──────┴─────────╯ │                           │ │
            │                    │ │                                  │ │ shape_globpattern          │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_int                  │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_internalcall         │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_list                 │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_literal              │ blue               │                           │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                           │ │
            │                    │ │                                  │ │ shape_matching_brackets    │ │ attr │ u │       │                           │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                           │ │
            │                    │ │                                  │ │ shape_nothing              │ light_cyan         │                           │ │
            │                    │ │                                  │ │ shape_operator             │ yellow             │                           │ │
            │                    │ │                                  │ │ shape_or                   │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_pipe                 │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_range                │ yellow_bold        │                           │ │
            │                    │ │                                  │ │ shape_record               │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_redirection          │ purple_bold        │                           │ │
            │                    │ │                                  │ │ shape_signature            │ green_bold         │                           │ │
            │                    │ │                                  │ │ shape_string               │ green              │                           │ │
            │                    │ │                                  │ │ shape_string_interpolation │ cyan_bold          │                           │ │
            │                    │ │                                  │ │ shape_table                │ blue_bold          │                           │ │
            │                    │ │                                  │ │ shape_variable             │ purple             │                           │ │
            │                    │ │                                  │ ╰────────────────────────────┴────────────────────╯                           │ │
            │                    │ │ footer_mode                      │ 25                                                                            │ │
            │                    │ │ float_precision                  │ 2                                                                             │ │
            │                    │ │ use_ansi_coloring                │ true                                                                          │ │
            │                    │ │ edit_mode                        │ emacs                                                                         │ │
            │                    │ │ shell_integration                │ true                                                                          │ │
            │                    │ │ show_banner                      │ true                                                                          │ │
            │                    │ │ render_right_prompt_on_last_line │ false                                                                         │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────╮                                       │ │
            │                    │ │ hooks                            │ │                │ ╭───┬──╮           │                                       │ │
            │                    │ │                                  │ │ pre_prompt     │ │ 0 │  │           │                                       │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                       │ │
            │                    │ │                                  │ │                │ ╭───┬──╮           │                                       │ │
            │                    │ │                                  │ │ pre_execution  │ │ 0 │  │           │                                       │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                       │ │
            │                    │ │                                  │ │                │ ╭─────┬──────────╮ │                                       │ │
            │                    │ │                                  │ │ env_change     │ │     │ ╭───┬──╮ │ │                                       │ │
            │                    │ │                                  │ │                │ │ PWD │ │ 0 │  │ │ │                                       │ │
            │                    │ │                                  │ │                │ │     │ ╰───┴──╯ │ │                                       │ │
            │                    │ │                                  │ │                │ ╰─────┴──────────╯ │                                       │ │
            │                    │ │                                  │ │ display_output │                    │                                       │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────╯                                       │ │
            │                    │ │                                  │ ╭───┬───────────────────────────┬────────────────────────┬────────┬───┬─────╮ │ │
            │                    │ │ menus                            │ │ # │           name            │ only_buffer_difference │ marker │ t │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ y │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ p │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ ├───┼───────────────────────────┼────────────────────────┼────────┼───┼─────┤ │ │
            │                    │ │                                  │ │ 0 │ completion_menu           │ false                  │ |      │ { │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ c │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ o │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ 4 │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ f │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ i │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ l │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ s │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ } │     │ │ │
            │                    │ │                                  │ │ 1 │ history_menu              │ true                   │ ?      │ { │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ c │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ o │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ 2 │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ f │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ i │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ l │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ s │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ } │     │ │ │
            │                    │ │                                  │ │ 2 │ help_menu                 │ true                   │ ?      │ { │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ c │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ o │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ 6 │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ f │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ i │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ l │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ s │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ } │     │ │ │
            │                    │ │                                  │ │ 3 │ commands_menu             │ false                  │ #      │ { │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ c │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ o │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ 4 │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ f │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ i │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ l │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ s │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ } │     │ │ │
            │                    │ │                                  │ │ 4 │ vars_menu                 │ true                   │ #      │ { │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ c │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ o │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ 2 │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ f │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ i │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ l │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ s │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ } │     │ │ │
            │                    │ │                                  │ │ 5 │ commands_with_description │ true                   │ #      │ { │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ c │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ o │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ r │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ 6 │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │   │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ f │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ i │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ e │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ l │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ d │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ s │     │ │ │
            │                    │ │                                  │ │   │                           │                        │        │ } │     │ │ │
            │                    │ │                                  │ ╰───┴───────────────────────────┴────────────────────────┴────────┴───┴─────╯ │ │
            │                    │ │                                  │ ╭────┬───────────────────────────┬──────────┬─────────┬─────────────────┬───╮ │ │
            │                    │ │ keybindings                      │ │  # │           name            │ modifier │ keycode │      mode       │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ v │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ n │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ t │ │ │
            │                    │ │                                  │ ├────┼───────────────────────────┼──────────┼─────────┼─────────────────┼───┤ │ │
            │                    │ │                                  │ │  0 │ completion_menu           │ none     │ tab     │ ╭───┬─────────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs   │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_norm │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ al      │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_inse │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ rt      │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴─────────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  1 │ completion_previous       │ shift    │ backtab │ ╭───┬─────────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs   │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_norm │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ al      │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_inse │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ rt      │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴─────────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  2 │ history_menu              │ control  │ char_r  │ emacs           │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  3 │ next_page                 │ control  │ char_x  │ emacs           │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  4 │ undo_or_previous_page     │ control  │ char_z  │ emacs           │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  5 │ yank                      │ control  │ char_y  │ emacs           │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  6 │ unix-line-discard         │ control  │ char_u  │ ╭───┬─────────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs   │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_norm │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ al      │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_inse │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ rt      │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴─────────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  7 │ kill-line                 │ control  │ char_k  │ ╭───┬─────────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs   │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_norm │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ al      │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_inse │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ rt      │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴─────────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  8 │ commands_menu             │ control  │ char_t  │ ╭───┬─────────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs   │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_norm │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ al      │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_inse │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ rt      │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴─────────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │  9 │ vars_menu                 │ alt      │ char_o  │ ╭───┬─────────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs   │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_norm │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ al      │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_inse │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ rt      │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴─────────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ │ 10 │ commands_with_description │ control  │ char_s  │ ╭───┬─────────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ emacs   │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi_norm │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ al      │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi_inse │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ rt      │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴─────────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │                 │ } │ │ │
            │                    │ │                                  │ ╰────┴───────────────────────────┴──────────┴─────────┴─────────────────┴───╯ │ │
            │                    │ ╰──────────────────────────────────┴───────────────────────────────────────────────────────────────────────────────╯ │
            ╰────────────────────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯"#}
    );
    Ok(())
}

#[test]
fn table_expande_with_no_header_internally_1() -> Result {
    let nu_value = r##"{ "config            ": { "ls": { "use_ls_colors": true, "clickable_links": false }, "rm": { "always_trash": false }, "cd": { "abbreviations": false }, "table": { "mode": "rounded", "index_mode": "always", "trim": { "methodology": "wrapping", "wrapping_try_keep_words": true, "truncating_suffix": "..." } }, "explore": { "help_banner": true, "exit_esc": true, "command_bar_text": "#C4C9C6", "status_bar_background": { "fg": "#1D1F21", "bg": "#C4C9C6" }, "highlight": { "bg": "yellow", "fg": "black" }, "status": {}, "try": {}, "table": { "split_line": "#404040", "cursor": true, "line_index": true, "line_shift": true, "line_head_top": true, "line_head_bottom": true, "show_head": true, "show_index": true }, "config": { "cursor_color": { "bg": "yellow", "fg": "black" } } }, "history": { "max_size": 10000, "sync_on_enter": true, "file_format": "plaintext" }, "completions": { "case_sensitive": false, "quick": true, "partial": true, "algorithm": "prefix", "external": { "enable": true, "max_results": 100, "completer": null } }, "filesize": { "metric": true, "format": "auto" }, "cursor_shape": { "emacs": "line", "vi_insert": "block", "vi_normal": "underscore" }, "color_config": { "separator": "default", "leading_trailing_space_bg": { "attr": "n" }, "header": "green_bold", "empty": "blue", "bool": null, "int": "default", "filesize": null, "duration": "default", "datetime": null, "range": "default", "float": "default", "string": "default", "nothing": "default", "binary": "default", "cell-path": "default", "row_index": "green_bold", "record": "default", "list": "default", "block": "default", "hints": "dark_gray", "search_result": {"fg": "white", "bg": "red"}, "shape_and": "purple_bold", "shape_binary": "purple_bold", "shape_block": "blue_bold", "shape_bool": "light_cyan", "shape_custom": "green", "shape_datetime": "cyan_bold", "shape_directory": "cyan", "shape_external": "cyan", "shape_externalarg": "green_bold", "shape_filepath": "cyan", "shape_flag": "blue_bold", "shape_float": "purple_bold", "shape_garbage": { "fg": "#FFFFFF", "bg": "#FF0000", "attr": "b" }, "shape_globpattern": "cyan_bold", "shape_int": "purple_bold", "shape_internalcall": "cyan_bold", "shape_list": "cyan_bold", "shape_literal": "blue", "shape_matching_brackets": { "attr": "u" }, "shape_nothing": "light_cyan", "shape_operator": "yellow", "shape_or": "purple_bold", "shape_pipe": "purple_bold", "shape_range": "yellow_bold", "shape_record": "cyan_bold", "shape_redirection": "purple_bold", "shape_signature": "green_bold", "shape_string": "green", "shape_string_interpolation": "cyan_bold", "shape_table": "blue_bold", "shape_variable": "purple" }, "footer_mode": "25", "float_precision": 2, "use_ansi_coloring": true, "edit_mode": "emacs", "shell_integration": true, "show_banner": true, "render_right_prompt_on_last_line": false, "hooks": { "pre_prompt": [ null ], "pre_execution": [ null ], "env_change": { "PWD": [ null ] }, "display_output": null }, "menus": [ { "name": "completion_menu", "only_buffer_difference": false, "marker": "| ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "history_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "help_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "commands_menu", "only_buffer_difference": false, "marker": "# ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "vars_menu", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "commands_with_description", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null } ], "keybindings": [ { "name": "completion_menu", "modifier": "none", "keycode": "tab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "send": "menu", "name": "completion_menu" }, { "send": "menunext" } ] } }, { "name": "completion_previous", "modifier": "shift", "keycode": "backtab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menuprevious" } }, { "name": "history_menu", "modifier": "control", "keycode": "char_r", "mode": "emacs", "event": { "send": "menu", "name": "history_menu" } }, { "name": "next_page", "modifier": "control", "keycode": "char_x", "mode": "emacs", "event": { "send": "menupagenext" } }, { "name": "undo_or_previous_page", "modifier": "control", "keycode": "char_z", "mode": "emacs", "event": { "until": [ { "send": "menupageprevious" }, { "edit": "undo" } ] } }, { "name": "yank", "modifier": "control", "keycode": "char_y", "mode": "emacs", "event": { "until": [ { "edit": "pastecutbufferafter" } ] } }, { "name": "unix-line-discard", "modifier": "control", "keycode": "char_u", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cutfromlinestart" } ] } }, { "name": "kill-line", "modifier": "control", "keycode": "char_k", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cuttolineend" } ] } }, { "name": "commands_menu", "modifier": "control", "keycode": "char_t", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_menu" } }, { "name": "vars_menu", "modifier": "alt", "keycode": "char_o", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "vars_menu" } }, { "name": "commands_with_description", "modifier": "control", "keycode": "char_s", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_with_description" } } ] } }"##;

    let mut tester = test();
    let data: Value = tester.run(nu_value.trim())?;
    let actual: String = tester.run_with_data("table --expand --width 136", data)?;

    assert_eq!(
        actual,
        indoc! {r#"
            ╭────────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                    │ ╭──────────────────────────────────┬──────────────────────────────────────────────────────────────────────────╮ │
            │ config             │ │                                  │ ╭─────────────────┬───────╮                                              │ │
            │                    │ │ ls                               │ │ use_ls_colors   │ true  │                                              │ │
            │                    │ │                                  │ │ clickable_links │ false │                                              │ │
            │                    │ │                                  │ ╰─────────────────┴───────╯                                              │ │
            │                    │ │                                  │ ╭──────────────┬───────╮                                                 │ │
            │                    │ │ rm                               │ │ always_trash │ false │                                                 │ │
            │                    │ │                                  │ ╰──────────────┴───────╯                                                 │ │
            │                    │ │                                  │ ╭───────────────┬───────╮                                                │ │
            │                    │ │ cd                               │ │ abbreviations │ false │                                                │ │
            │                    │ │                                  │ ╰───────────────┴───────╯                                                │ │
            │                    │ │                                  │ ╭────────────┬────────────────────────────────────────╮                  │ │
            │                    │ │ table                            │ │ mode       │ rounded                                │                  │ │
            │                    │ │                                  │ │ index_mode │ always                                 │                  │ │
            │                    │ │                                  │ │            │ ╭─────────────────────────┬──────────╮ │                  │ │
            │                    │ │                                  │ │ trim       │ │ methodology             │ wrapping │ │                  │ │
            │                    │ │                                  │ │            │ │ wrapping_try_keep_words │ true     │ │                  │ │
            │                    │ │                                  │ │            │ │ truncating_suffix       │ ...      │ │                  │ │
            │                    │ │                                  │ │            │ ╰─────────────────────────┴──────────╯ │                  │ │
            │                    │ │                                  │ ╰────────────┴────────────────────────────────────────╯                  │ │
            │                    │ │                                  │ ╭────────────────────────────┬─────────────────────────────────────────╮ │ │
            │                    │ │ explore                          │ │ help_banner                │ true                                    │ │ │
            │                    │ │                                  │ │ exit_esc                   │ true                                    │ │ │
            │                    │ │                                  │ │ command_bar_text           │ #C4C9C6                                 │ │ │
            │                    │ │                                  │ │                            │ ╭────┬─────────╮                        │ │ │
            │                    │ │                                  │ │ status_bar_background      │ │ fg │ #1D1F21 │                        │ │ │
            │                    │ │                                  │ │                            │ │ bg │ #C4C9C6 │                        │ │ │
            │                    │ │                                  │ │                            │ ╰────┴─────────╯                        │ │ │
            │                    │ │                                  │ │                            │ ╭────┬────────╮                         │ │ │
            │                    │ │                                  │ │ highlight                  │ │ bg │ yellow │                         │ │ │
            │                    │ │                                  │ │                            │ │ fg │ black  │                         │ │ │
            │                    │ │                                  │ │                            │ ╰────┴────────╯                         │ │ │
            │                    │ │                                  │ │ status                     │ {record 0 fields}                       │ │ │
            │                    │ │                                  │ │ try                        │ {record 0 fields}                       │ │ │
            │                    │ │                                  │ │                            │ ╭──────────────────┬─────────╮          │ │ │
            │                    │ │                                  │ │ table                      │ │ split_line       │ #404040 │          │ │ │
            │                    │ │                                  │ │                            │ │ cursor           │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_index       │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_shift       │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_head_top    │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ line_head_bottom │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ show_head        │ true    │          │ │ │
            │                    │ │                                  │ │                            │ │ show_index       │ true    │          │ │ │
            │                    │ │                                  │ │                            │ ╰──────────────────┴─────────╯          │ │ │
            │                    │ │                                  │ │                            │ ╭──────────────┬─────────────────╮      │ │ │
            │                    │ │                                  │ │ config                     │ │              │ ╭────┬────────╮ │      │ │ │
            │                    │ │                                  │ │                            │ │ cursor_color │ │ bg │ yellow │ │      │ │ │
            │                    │ │                                  │ │                            │ │              │ │ fg │ black  │ │      │ │ │
            │                    │ │                                  │ │                            │ │              │ ╰────┴────────╯ │      │ │ │
            │                    │ │                                  │ │                            │ ╰──────────────┴─────────────────╯      │ │ │
            │                    │ │                                  │ ╰────────────────────────────┴─────────────────────────────────────────╯ │ │
            │                    │ │                                  │ ╭───────────────┬───────────╮                                            │ │
            │                    │ │ history                          │ │ max_size      │ 10000     │                                            │ │
            │                    │ │                                  │ │ sync_on_enter │ true      │                                            │ │
            │                    │ │                                  │ │ file_format   │ plaintext │                                            │ │
            │                    │ │                                  │ ╰───────────────┴───────────╯                                            │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────────╮                              │ │
            │                    │ │ completions                      │ │ case_sensitive │ false                  │                              │ │
            │                    │ │                                  │ │ quick          │ true                   │                              │ │
            │                    │ │                                  │ │ partial        │ true                   │                              │ │
            │                    │ │                                  │ │ algorithm      │ prefix                 │                              │ │
            │                    │ │                                  │ │                │ ╭─────────────┬──────╮ │                              │ │
            │                    │ │                                  │ │ external       │ │ enable      │ true │ │                              │ │
            │                    │ │                                  │ │                │ │ max_results │ 100  │ │                              │ │
            │                    │ │                                  │ │                │ │ completer   │      │ │                              │ │
            │                    │ │                                  │ │                │ ╰─────────────┴──────╯ │                              │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────────╯                              │ │
            │                    │ │                                  │ ╭────────┬──────╮                                                        │ │
            │                    │ │ filesize                         │ │ metric │ true │                                                        │ │
            │                    │ │                                  │ │ format │ auto │                                                        │ │
            │                    │ │                                  │ ╰────────┴──────╯                                                        │ │
            │                    │ │                                  │ ╭───────────┬────────────╮                                               │ │
            │                    │ │ cursor_shape                     │ │ emacs     │ line       │                                               │ │
            │                    │ │                                  │ │ vi_insert │ block      │                                               │ │
            │                    │ │                                  │ │ vi_normal │ underscore │                                               │ │
            │                    │ │                                  │ ╰───────────┴────────────╯                                               │ │
            │                    │ │                                  │ ╭────────────────────────────┬────────────────────╮                      │ │
            │                    │ │ color_config                     │ │ separator                  │ default            │                      │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                      │ │
            │                    │ │                                  │ │ leading_trailing_space_bg  │ │ attr │ n │       │                      │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                      │ │
            │                    │ │                                  │ │ header                     │ green_bold         │                      │ │
            │                    │ │                                  │ │ empty                      │ blue               │                      │ │
            │                    │ │                                  │ │ bool                       │                    │                      │ │
            │                    │ │                                  │ │ int                        │ default            │                      │ │
            │                    │ │                                  │ │ filesize                   │                    │                      │ │
            │                    │ │                                  │ │ duration                   │ default            │                      │ │
            │                    │ │                                  │ │ datetime                   │                    │                      │ │
            │                    │ │                                  │ │ range                      │ default            │                      │ │
            │                    │ │                                  │ │ float                      │ default            │                      │ │
            │                    │ │                                  │ │ string                     │ default            │                      │ │
            │                    │ │                                  │ │ nothing                    │ default            │                      │ │
            │                    │ │                                  │ │ binary                     │ default            │                      │ │
            │                    │ │                                  │ │ cell-path                  │ default            │                      │ │
            │                    │ │                                  │ │ row_index                  │ green_bold         │                      │ │
            │                    │ │                                  │ │ record                     │ default            │                      │ │
            │                    │ │                                  │ │ list                       │ default            │                      │ │
            │                    │ │                                  │ │ block                      │ default            │                      │ │
            │                    │ │                                  │ │ hints                      │ dark_gray          │                      │ │
            │                    │ │                                  │ │                            │ ╭────┬───────╮     │                      │ │
            │                    │ │                                  │ │ search_result              │ │ fg │ white │     │                      │ │
            │                    │ │                                  │ │                            │ │ bg │ red   │     │                      │ │
            │                    │ │                                  │ │                            │ ╰────┴───────╯     │                      │ │
            │                    │ │                                  │ │ shape_and                  │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_binary               │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_block                │ blue_bold          │                      │ │
            │                    │ │                                  │ │ shape_bool                 │ light_cyan         │                      │ │
            │                    │ │                                  │ │ shape_custom               │ green              │                      │ │
            │                    │ │                                  │ │ shape_datetime             │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_directory            │ cyan               │                      │ │
            │                    │ │                                  │ │ shape_external             │ cyan               │                      │ │
            │                    │ │                                  │ │ shape_externalarg          │ green_bold         │                      │ │
            │                    │ │                                  │ │ shape_filepath             │ cyan               │                      │ │
            │                    │ │                                  │ │ shape_flag                 │ blue_bold          │                      │ │
            │                    │ │                                  │ │ shape_float                │ purple_bold        │                      │ │
            │                    │ │                                  │ │                            │ ╭──────┬─────────╮ │                      │ │
            │                    │ │                                  │ │ shape_garbage              │ │ fg   │ #FFFFFF │ │                      │ │
            │                    │ │                                  │ │                            │ │ bg   │ #FF0000 │ │                      │ │
            │                    │ │                                  │ │                            │ │ attr │ b       │ │                      │ │
            │                    │ │                                  │ │                            │ ╰──────┴─────────╯ │                      │ │
            │                    │ │                                  │ │ shape_globpattern          │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_int                  │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_internalcall         │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_list                 │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_literal              │ blue               │                      │ │
            │                    │ │                                  │ │                            │ ╭──────┬───╮       │                      │ │
            │                    │ │                                  │ │ shape_matching_brackets    │ │ attr │ u │       │                      │ │
            │                    │ │                                  │ │                            │ ╰──────┴───╯       │                      │ │
            │                    │ │                                  │ │ shape_nothing              │ light_cyan         │                      │ │
            │                    │ │                                  │ │ shape_operator             │ yellow             │                      │ │
            │                    │ │                                  │ │ shape_or                   │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_pipe                 │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_range                │ yellow_bold        │                      │ │
            │                    │ │                                  │ │ shape_record               │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_redirection          │ purple_bold        │                      │ │
            │                    │ │                                  │ │ shape_signature            │ green_bold         │                      │ │
            │                    │ │                                  │ │ shape_string               │ green              │                      │ │
            │                    │ │                                  │ │ shape_string_interpolation │ cyan_bold          │                      │ │
            │                    │ │                                  │ │ shape_table                │ blue_bold          │                      │ │
            │                    │ │                                  │ │ shape_variable             │ purple             │                      │ │
            │                    │ │                                  │ ╰────────────────────────────┴────────────────────╯                      │ │
            │                    │ │ footer_mode                      │ 25                                                                       │ │
            │                    │ │ float_precision                  │ 2                                                                        │ │
            │                    │ │ use_ansi_coloring                │ true                                                                     │ │
            │                    │ │ edit_mode                        │ emacs                                                                    │ │
            │                    │ │ shell_integration                │ true                                                                     │ │
            │                    │ │ show_banner                      │ true                                                                     │ │
            │                    │ │ render_right_prompt_on_last_line │ false                                                                    │ │
            │                    │ │                                  │ ╭────────────────┬────────────────────╮                                  │ │
            │                    │ │ hooks                            │ │                │ ╭───┬──╮           │                                  │ │
            │                    │ │                                  │ │ pre_prompt     │ │ 0 │  │           │                                  │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                  │ │
            │                    │ │                                  │ │                │ ╭───┬──╮           │                                  │ │
            │                    │ │                                  │ │ pre_execution  │ │ 0 │  │           │                                  │ │
            │                    │ │                                  │ │                │ ╰───┴──╯           │                                  │ │
            │                    │ │                                  │ │                │ ╭─────┬──────────╮ │                                  │ │
            │                    │ │                                  │ │ env_change     │ │     │ ╭───┬──╮ │ │                                  │ │
            │                    │ │                                  │ │                │ │ PWD │ │ 0 │  │ │ │                                  │ │
            │                    │ │                                  │ │                │ │     │ ╰───┴──╯ │ │                                  │ │
            │                    │ │                                  │ │                │ ╰─────┴──────────╯ │                                  │ │
            │                    │ │                                  │ │ display_output │                    │                                  │ │
            │                    │ │                                  │ ╰────────────────┴────────────────────╯                                  │ │
            │                    │ │                                  │ ╭───┬───────────────────────────┬────────────────────────┬───────┬─────╮ │ │
            │                    │ │ menus                            │ │ # │           name            │ only_buffer_difference │ marke │ ... │ │ │
            │                    │ │                                  │ │   │                           │                        │ r     │     │ │ │
            │                    │ │                                  │ ├───┼───────────────────────────┼────────────────────────┼───────┼─────┤ │ │
            │                    │ │                                  │ │ 0 │ completion_menu           │ false                  │ |     │ ... │ │ │
            │                    │ │                                  │ │ 1 │ history_menu              │ true                   │ ?     │ ... │ │ │
            │                    │ │                                  │ │ 2 │ help_menu                 │ true                   │ ?     │ ... │ │ │
            │                    │ │                                  │ │ 3 │ commands_menu             │ false                  │ #     │ ... │ │ │
            │                    │ │                                  │ │ 4 │ vars_menu                 │ true                   │ #     │ ... │ │ │
            │                    │ │                                  │ │ 5 │ commands_with_description │ true                   │ #     │ ... │ │ │
            │                    │ │                                  │ ╰───┴───────────────────────────┴────────────────────────┴───────┴─────╯ │ │
            │                    │ │                                  │ ╭────┬───────────────────────────┬──────────┬─────────┬────────────┬───╮ │ │
            │                    │ │ keybindings                      │ │  # │           name            │ modifier │ keycode │    mode    │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ v │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ n │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ t │ │ │
            │                    │ │                                  │ ├────┼───────────────────────────┼──────────┼─────────┼────────────┼───┤ │ │
            │                    │ │                                  │ │  0 │ completion_menu           │ none     │ tab     │ ╭───┬────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ em │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ac │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s  │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _n │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ or │ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ma │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l  │ │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _i │ │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ns │ │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ er │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t  │ │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  1 │ completion_previous       │ shift    │ backtab │ ╭───┬────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ em │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ac │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s  │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _n │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ or │ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ma │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l  │ │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _i │ │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ns │ │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ er │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t  │ │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  2 │ history_menu              │ control  │ char_r  │ emacs      │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  3 │ next_page                 │ control  │ char_x  │ emacs      │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  4 │ undo_or_previous_page     │ control  │ char_z  │ emacs      │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  5 │ yank                      │ control  │ char_y  │ emacs      │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  6 │ unix-line-discard         │ control  │ char_u  │ ╭───┬────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ em │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ac │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s  │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _n │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ or │ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ma │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l  │ │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _i │ │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ns │ │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ er │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t  │ │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  7 │ kill-line                 │ control  │ char_k  │ ╭───┬────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ em │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ac │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s  │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _n │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ or │ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ma │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l  │ │ 1 │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _i │ │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ns │ │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ er │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t  │ │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  8 │ commands_menu             │ control  │ char_t  │ ╭───┬────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ em │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ac │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s  │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _n │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ or │ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ma │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l  │ │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _i │ │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ns │ │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ er │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t  │ │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │  9 │ vars_menu                 │ alt      │ char_o  │ ╭───┬────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ em │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ac │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s  │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _n │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ or │ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ma │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l  │ │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _i │ │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ns │ │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ er │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t  │ │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ │ 10 │ commands_with_description │ control  │ char_s  │ ╭───┬────╮ │ { │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 0 │ em │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ac │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ s  │ │ c │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 1 │ vi │ │ o │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _n │ │ r │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ or │ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ma │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ l  │ │ 2 │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │ 2 │ vi │ │   │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ _i │ │ f │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ ns │ │ i │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ er │ │ e │ │ │
            │                    │ │                                  │ │    │                           │          │         │ │   │ t  │ │ l │ │ │
            │                    │ │                                  │ │    │                           │          │         │ ╰───┴────╯ │ d │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ s │ │ │
            │                    │ │                                  │ │    │                           │          │         │            │ } │ │ │
            │                    │ │                                  │ ╰────┴───────────────────────────┴──────────┴─────────┴────────────┴───╯ │ │
            │                    │ ╰──────────────────────────────────┴──────────────────────────────────────────────────────────────────────────╯ │
            ╰────────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯"#}
    );
    Ok(())
}

#[test]
fn big_table_expanded_with_padding_0() -> Result {
    let nu_value = r##"{ "config            ": { "ls": { "use_ls_colors": true, "clickable_links": false }, "rm": { "always_trash": false }, "cd": { "abbreviations": false }, "table": { "mode": "rounded", "index_mode": "always", "trim": { "methodology": "wrapping", "wrapping_try_keep_words": true, "truncating_suffix": "..." } }, "explore": { "help_banner": true, "exit_esc": true, "command_bar_text": "#C4C9C6", "status_bar_background": { "fg": "#1D1F21", "bg": "#C4C9C6" }, "highlight": { "bg": "yellow", "fg": "black" }, "status": {}, "try": {}, "table": { "split_line": "#404040", "cursor": true, "line_index": true, "line_shift": true, "line_head_top": true, "line_head_bottom": true, "show_head": true, "show_index": true }, "config": { "cursor_color": { "bg": "yellow", "fg": "black" } } }, "history": { "max_size": 10000, "sync_on_enter": true, "file_format": "plaintext" }, "completions": { "case_sensitive": false, "quick": true, "partial": true, "algorithm": "prefix", "external": { "enable": true, "max_results": 100, "completer": null } }, "filesize": { "metric": true, "format": "auto" }, "cursor_shape": { "emacs": "line", "vi_insert": "block", "vi_normal": "underscore" }, "color_config": { "separator": "default", "leading_trailing_space_bg": { "attr": "n" }, "header": "green_bold", "empty": "blue", "bool": null, "int": "default", "filesize": null, "duration": "default", "datetime": null, "range": "default", "float": "default", "string": "default", "nothing": "default", "binary": "default", "cell-path": "default", "row_index": "green_bold", "record": "default", "list": "default", "block": "default", "hints": "dark_gray", "search_result": {"fg": "white", "bg": "red"}, "shape_and": "purple_bold", "shape_binary": "purple_bold", "shape_block": "blue_bold", "shape_bool": "light_cyan", "shape_custom": "green", "shape_datetime": "cyan_bold", "shape_directory": "cyan", "shape_external": "cyan", "shape_externalarg": "green_bold", "shape_filepath": "cyan", "shape_flag": "blue_bold", "shape_float": "purple_bold", "shape_garbage": { "fg": "#FFFFFF", "bg": "#FF0000", "attr": "b" }, "shape_globpattern": "cyan_bold", "shape_int": "purple_bold", "shape_internalcall": "cyan_bold", "shape_list": "cyan_bold", "shape_literal": "blue", "shape_matching_brackets": { "attr": "u" }, "shape_nothing": "light_cyan", "shape_operator": "yellow", "shape_or": "purple_bold", "shape_pipe": "purple_bold", "shape_range": "yellow_bold", "shape_record": "cyan_bold", "shape_redirection": "purple_bold", "shape_signature": "green_bold", "shape_string": "green", "shape_string_interpolation": "cyan_bold", "shape_table": "blue_bold", "shape_variable": "purple" }, "footer_mode": "25", "float_precision": 2, "use_ansi_coloring": true, "edit_mode": "emacs", "shell_integration": true, "show_banner": true, "render_right_prompt_on_last_line": false, "hooks": { "pre_prompt": [ null ], "pre_execution": [ null ], "env_change": { "PWD": [ null ] }, "display_output": null }, "menus": [ { "name": "completion_menu", "only_buffer_difference": false, "marker": "| ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "history_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "help_menu", "only_buffer_difference": true, "marker": "? ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" } }, { "name": "commands_menu", "only_buffer_difference": false, "marker": "# ", "type": { "layout": "columnar", "columns": 4, "col_width": 20, "col_padding": 2 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "vars_menu", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "list", "page_size": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null }, { "name": "commands_with_description", "only_buffer_difference": true, "marker": "# ", "type": { "layout": "description", "columns": 4, "col_width": 20, "col_padding": 2, "selection_rows": 4, "description_rows": 10 }, "style": { "text": "green", "selected_text": "green_reverse", "description_text": "yellow" }, "source": null } ], "keybindings": [ { "name": "completion_menu", "modifier": "none", "keycode": "tab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "send": "menu", "name": "completion_menu" }, { "send": "menunext" } ] } }, { "name": "completion_previous", "modifier": "shift", "keycode": "backtab", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menuprevious" } }, { "name": "history_menu", "modifier": "control", "keycode": "char_r", "mode": "emacs", "event": { "send": "menu", "name": "history_menu" } }, { "name": "next_page", "modifier": "control", "keycode": "char_x", "mode": "emacs", "event": { "send": "menupagenext" } }, { "name": "undo_or_previous_page", "modifier": "control", "keycode": "char_z", "mode": "emacs", "event": { "until": [ { "send": "menupageprevious" }, { "edit": "undo" } ] } }, { "name": "yank", "modifier": "control", "keycode": "char_y", "mode": "emacs", "event": { "until": [ { "edit": "pastecutbufferafter" } ] } }, { "name": "unix-line-discard", "modifier": "control", "keycode": "char_u", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cutfromlinestart" } ] } }, { "name": "kill-line", "modifier": "control", "keycode": "char_k", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "until": [ { "edit": "cuttolineend" } ] } }, { "name": "commands_menu", "modifier": "control", "keycode": "char_t", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_menu" } }, { "name": "vars_menu", "modifier": "alt", "keycode": "char_o", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "vars_menu" } }, { "name": "commands_with_description", "modifier": "control", "keycode": "char_s", "mode": [ "emacs", "vi_normal", "vi_insert" ], "event": { "send": "menu", "name": "commands_with_description" } } ] } }"##;

    let mut tester = test();
    let data: Value = tester.run(nu_value.trim())?;
    let actual: String = tester.run_with_data(
        "
            let data = $in
            $env.config.table.padding = { left: 2, right: 3 }
            $data
            | table --expand --width 141
        ",
        data,
    )?;

    assert_eq!(
        actual,
        indoc! {r#"
            ╭───────────────────────┬───────────────────────────────────────────────────────────────────────────────────────────────────────────────────╮
            │                       │  ╭─────────────────────────────────────┬──────────────────────────────────────────────────────────────────────╮   │
            │  config               │  │                                     │  ╭────────────────────┬──────────╮                                   │   │
            │                       │  │  ls                                 │  │  use_ls_colors     │  true    │                                   │   │
            │                       │  │                                     │  │  clickable_links   │  false   │                                   │   │
            │                       │  │                                     │  ╰────────────────────┴──────────╯                                   │   │
            │                       │  │                                     │  ╭─────────────────┬──────────╮                                      │   │
            │                       │  │  rm                                 │  │  always_trash   │  false   │                                      │   │
            │                       │  │                                     │  ╰─────────────────┴──────────╯                                      │   │
            │                       │  │                                     │  ╭──────────────────┬──────────╮                                     │   │
            │                       │  │  cd                                 │  │  abbreviations   │  false   │                                     │   │
            │                       │  │                                     │  ╰──────────────────┴──────────╯                                     │   │
            │                       │  │                                     │  ╭───────────────┬───────────────────────────────────────────────╮   │   │
            │                       │  │  table                              │  │  mode         │  rounded                                      │   │   │
            │                       │  │                                     │  │  index_mode   │  always                                       │   │   │
            │                       │  │                                     │  │               │  ╭────────────────────────────┬───────────╮   │   │   │
            │                       │  │                                     │  │  trim         │  │  methodology               │  wrappi   │   │   │   │
            │                       │  │                                     │  │               │  │                            │  ng       │   │   │   │
            │                       │  │                                     │  │               │  │  wrapping_try_keep_words   │  true     │   │   │   │
            │                       │  │                                     │  │               │  │  truncating_suffix         │  ...      │   │   │   │
            │                       │  │                                     │  │               │  ╰────────────────────────────┴───────────╯   │   │   │
            │                       │  │                                     │  ╰───────────────┴───────────────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭──────────────────────────┬────────────────────────────────────╮   │   │
            │                       │  │  explore                            │  │  help_banner             │  true                              │   │   │
            │                       │  │                                     │  │  exit_esc                │  true                              │   │   │
            │                       │  │                                     │  │  command_bar_text        │  #C4C9C6                           │   │   │
            │                       │  │                                     │  │                          │  ╭───────┬────────────╮            │   │   │
            │                       │  │                                     │  │  status_bar_background   │  │  fg   │  #1D1F21   │            │   │   │
            │                       │  │                                     │  │                          │  │  bg   │  #C4C9C6   │            │   │   │
            │                       │  │                                     │  │                          │  ╰───────┴────────────╯            │   │   │
            │                       │  │                                     │  │                          │  ╭───────┬───────────╮             │   │   │
            │                       │  │                                     │  │  highlight               │  │  bg   │  yellow   │             │   │   │
            │                       │  │                                     │  │                          │  │  fg   │  black    │             │   │   │
            │                       │  │                                     │  │                          │  ╰───────┴───────────╯             │   │   │
            │                       │  │                                     │  │  status                  │  {record 0 fields}                 │   │   │
            │                       │  │                                     │  │  try                     │  {record 0 fields}                 │   │   │
            │                       │  │                                     │  │                          │  ╭─────────────────────┬───────╮   │   │   │
            │                       │  │                                     │  │  table                   │  │  split_line         │  #4   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  04   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  04   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  0    │   │   │   │
            │                       │  │                                     │  │                          │  │  cursor             │  tr   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  ue   │   │   │   │
            │                       │  │                                     │  │                          │  │  line_index         │  tr   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  ue   │   │   │   │
            │                       │  │                                     │  │                          │  │  line_shift         │  tr   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  ue   │   │   │   │
            │                       │  │                                     │  │                          │  │  line_head_top      │  tr   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  ue   │   │   │   │
            │                       │  │                                     │  │                          │  │  line_head_bottom   │  tr   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  ue   │   │   │   │
            │                       │  │                                     │  │                          │  │  show_head          │  tr   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  ue   │   │   │   │
            │                       │  │                                     │  │                          │  │  show_index         │  tr   │   │   │   │
            │                       │  │                                     │  │                          │  │                     │  ue   │   │   │   │
            │                       │  │                                     │  │                          │  ╰─────────────────────┴───────╯   │   │   │
            │                       │  │                                     │  │                          │  ╭─────────────────┬───────────╮   │   │   │
            │                       │  │                                     │  │  config                  │  │  cursor_color   │  {recor   │   │   │   │
            │                       │  │                                     │  │                          │  │                 │  d 2 fi   │   │   │   │
            │                       │  │                                     │  │                          │  │                 │  elds}    │   │   │   │
            │                       │  │                                     │  │                          │  ╰─────────────────┴───────────╯   │   │   │
            │                       │  │                                     │  ╰──────────────────────────┴────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭──────────────────┬──────────────╮                                 │   │
            │                       │  │  history                            │  │  max_size        │  10000       │                                 │   │
            │                       │  │                                     │  │  sync_on_enter   │  true        │                                 │   │
            │                       │  │                                     │  │  file_format     │  plaintext   │                                 │   │
            │                       │  │                                     │  ╰──────────────────┴──────────────╯                                 │   │
            │                       │  │                                     │  ╭────────────────────────┬──────────────────────────────────────╮   │   │
            │                       │  │  completions                        │  │  case_sensitive        │  false                               │   │   │
            │                       │  │                                     │  │  quick                 │  true                                │   │   │
            │                       │  │                                     │  │  partial               │  true                                │   │   │
            │                       │  │                                     │  │  algorithm             │  prefix                              │   │   │
            │                       │  │                                     │  │                        │  ╭────────────────┬─────────╮        │   │   │
            │                       │  │                                     │  │  external              │  │  enable        │  true   │        │   │   │
            │                       │  │                                     │  │                        │  │  max_results   │  100    │        │   │   │
            │                       │  │                                     │  │                        │  │  completer     │         │        │   │   │
            │                       │  │                                     │  │                        │  ╰────────────────┴─────────╯        │   │   │
            │                       │  │                                     │  ╰────────────────────────┴──────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭───────────┬─────────╮                                             │   │
            │                       │  │  filesize                           │  │  metric   │  true   │                                             │   │
            │                       │  │                                     │  │  format   │  auto   │                                             │   │
            │                       │  │                                     │  ╰───────────┴─────────╯                                             │   │
            │                       │  │                                     │  ╭──────────────┬───────────────╮                                    │   │
            │                       │  │  cursor_shape                       │  │  emacs       │  line         │                                    │   │
            │                       │  │                                     │  │  vi_insert   │  block        │                                    │   │
            │                       │  │                                     │  │  vi_normal   │  underscore   │                                    │   │
            │                       │  │                                     │  ╰──────────────┴───────────────╯                                    │   │
            │                       │  │                                     │  ╭───────────────────────────────┬───────────────────────────────╮   │   │
            │                       │  │  color_config                       │  │  separator                    │  default                      │   │   │
            │                       │  │                                     │  │                               │  ╭─────────┬──────╮           │   │   │
            │                       │  │                                     │  │  leading_trailing_space_bg    │  │  attr   │  n   │           │   │   │
            │                       │  │                                     │  │                               │  ╰─────────┴──────╯           │   │   │
            │                       │  │                                     │  │  header                       │  green_bold                   │   │   │
            │                       │  │                                     │  │  empty                        │  blue                         │   │   │
            │                       │  │                                     │  │  bool                         │                               │   │   │
            │                       │  │                                     │  │  int                          │  default                      │   │   │
            │                       │  │                                     │  │  filesize                     │                               │   │   │
            │                       │  │                                     │  │  duration                     │  default                      │   │   │
            │                       │  │                                     │  │  datetime                     │                               │   │   │
            │                       │  │                                     │  │  range                        │  default                      │   │   │
            │                       │  │                                     │  │  float                        │  default                      │   │   │
            │                       │  │                                     │  │  string                       │  default                      │   │   │
            │                       │  │                                     │  │  nothing                      │  default                      │   │   │
            │                       │  │                                     │  │  binary                       │  default                      │   │   │
            │                       │  │                                     │  │  cell-path                    │  default                      │   │   │
            │                       │  │                                     │  │  row_index                    │  green_bold                   │   │   │
            │                       │  │                                     │  │  record                       │  default                      │   │   │
            │                       │  │                                     │  │  list                         │  default                      │   │   │
            │                       │  │                                     │  │  block                        │  default                      │   │   │
            │                       │  │                                     │  │  hints                        │  dark_gray                    │   │   │
            │                       │  │                                     │  │                               │  ╭───────┬──────────╮         │   │   │
            │                       │  │                                     │  │  search_result                │  │  fg   │  white   │         │   │   │
            │                       │  │                                     │  │                               │  │  bg   │  red     │         │   │   │
            │                       │  │                                     │  │                               │  ╰───────┴──────────╯         │   │   │
            │                       │  │                                     │  │  shape_and                    │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_binary                 │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_block                  │  blue_bold                    │   │   │
            │                       │  │                                     │  │  shape_bool                   │  light_cyan                   │   │   │
            │                       │  │                                     │  │  shape_custom                 │  green                        │   │   │
            │                       │  │                                     │  │  shape_datetime               │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_directory              │  cyan                         │   │   │
            │                       │  │                                     │  │  shape_external               │  cyan                         │   │   │
            │                       │  │                                     │  │  shape_externalarg            │  green_bold                   │   │   │
            │                       │  │                                     │  │  shape_filepath               │  cyan                         │   │   │
            │                       │  │                                     │  │  shape_flag                   │  blue_bold                    │   │   │
            │                       │  │                                     │  │  shape_float                  │  purple_bold                  │   │   │
            │                       │  │                                     │  │                               │  ╭──────────┬─────────────╮   │   │   │
            │                       │  │                                     │  │  shape_garbage                │  │  fg      │  #FFFFFF    │   │   │   │
            │                       │  │                                     │  │                               │  │  bg      │  #FF0000    │   │   │   │
            │                       │  │                                     │  │                               │  │  attr    │  b          │   │   │   │
            │                       │  │                                     │  │                               │  ╰──────────┴─────────────╯   │   │   │
            │                       │  │                                     │  │  shape_globpattern            │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_int                    │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_internalcall           │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_list                   │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_literal                │  blue                         │   │   │
            │                       │  │                                     │  │                               │  ╭─────────┬──────╮           │   │   │
            │                       │  │                                     │  │  shape_matching_brackets      │  │  attr   │  u   │           │   │   │
            │                       │  │                                     │  │                               │  ╰─────────┴──────╯           │   │   │
            │                       │  │                                     │  │  shape_nothing                │  light_cyan                   │   │   │
            │                       │  │                                     │  │  shape_operator               │  yellow                       │   │   │
            │                       │  │                                     │  │  shape_or                     │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_pipe                   │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_range                  │  yellow_bold                  │   │   │
            │                       │  │                                     │  │  shape_record                 │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_redirection            │  purple_bold                  │   │   │
            │                       │  │                                     │  │  shape_signature              │  green_bold                   │   │   │
            │                       │  │                                     │  │  shape_string                 │  green                        │   │   │
            │                       │  │                                     │  │  shape_string_interpolation   │  cyan_bold                    │   │   │
            │                       │  │                                     │  │  shape_table                  │  blue_bold                    │   │   │
            │                       │  │                                     │  │  shape_variable               │  purple                       │   │   │
            │                       │  │                                     │  ╰───────────────────────────────┴───────────────────────────────╯   │   │
            │                       │  │  footer_mode                        │  25                                                                  │   │
            │                       │  │  float_precision                    │  2                                                                   │   │
            │                       │  │  use_ansi_coloring                  │  true                                                                │   │
            │                       │  │  edit_mode                          │  emacs                                                               │   │
            │                       │  │  shell_integration                  │  true                                                                │   │
            │                       │  │  show_banner                        │  true                                                                │   │
            │                       │  │  render_right_prompt_on_last_line   │  false                                                               │   │
            │                       │  │                                     │  ╭───────────────────────┬───────────────────────────────────────╮   │   │
            │                       │  │  hooks                              │  │                       │  ╭──────┬─────╮                       │   │   │
            │                       │  │                                     │  │  pre_prompt           │  │  0   │     │                       │   │   │
            │                       │  │                                     │  │                       │  ╰──────┴─────╯                       │   │   │
            │                       │  │                                     │  │                       │  ╭──────┬─────╮                       │   │   │
            │                       │  │                                     │  │  pre_execution        │  │  0   │     │                       │   │   │
            │                       │  │                                     │  │                       │  ╰──────┴─────╯                       │   │   │
            │                       │  │                                     │  │                       │  ╭────────┬───────────────────╮       │   │   │
            │                       │  │                                     │  │  env_change           │  │        │  ╭──────┬─────╮   │       │   │   │
            │                       │  │                                     │  │                       │  │  PWD   │  │  0   │     │   │       │   │   │
            │                       │  │                                     │  │                       │  │        │  ╰──────┴─────╯   │       │   │   │
            │                       │  │                                     │  │                       │  ╰────────┴───────────────────╯       │   │   │
            │                       │  │                                     │  │  display_output       │                                       │   │   │
            │                       │  │                                     │  ╰───────────────────────┴───────────────────────────────────────╯   │   │
            │                       │  │                                     │  ╭──────┬──────────────────────────────┬────────────────┬────────╮   │   │
            │                       │  │  menus                              │  │  #   │            name              │  only_buffer   │  ...   │   │   │
            │                       │  │                                     │  │      │                              │  _difference   │        │   │   │
            │                       │  │                                     │  ├──────┼──────────────────────────────┼────────────────┼────────┤   │   │
            │                       │  │                                     │  │  0   │  completion_menu             │  false         │  ...   │   │   │
            │                       │  │                                     │  │  1   │  history_menu                │  true          │  ...   │   │   │
            │                       │  │                                     │  │  2   │  help_menu                   │  true          │  ...   │   │   │
            │                       │  │                                     │  │  3   │  commands_menu               │  false         │  ...   │   │   │
            │                       │  │                                     │  │  4   │  vars_menu                   │  true          │  ...   │   │   │
            │                       │  │                                     │  │  5   │  commands_with_description   │  true          │  ...   │   │   │
            │                       │  │                                     │  ╰──────┴──────────────────────────────┴────────────────┴────────╯   │   │
            │                       │  │                                     │  ╭───────┬──────────────────────────────┬─────────────┬────────╮     │   │
            │                       │  │  keybindings                        │  │   #   │            name              │  modifier   │  ...   │     │   │
            │                       │  │                                     │  ├───────┼──────────────────────────────┼─────────────┼────────┤     │   │
            │                       │  │                                     │  │   0   │  completion_menu             │  none       │  ...   │     │   │
            │                       │  │                                     │  │   1   │  completion_previous         │  shift      │  ...   │     │   │
            │                       │  │                                     │  │   2   │  history_menu                │  control    │  ...   │     │   │
            │                       │  │                                     │  │   3   │  next_page                   │  control    │  ...   │     │   │
            │                       │  │                                     │  │   4   │  undo_or_previous_page       │  control    │  ...   │     │   │
            │                       │  │                                     │  │   5   │  yank                        │  control    │  ...   │     │   │
            │                       │  │                                     │  │   6   │  unix-line-discard           │  control    │  ...   │     │   │
            │                       │  │                                     │  │   7   │  kill-line                   │  control    │  ...   │     │   │
            │                       │  │                                     │  │   8   │  commands_menu               │  control    │  ...   │     │   │
            │                       │  │                                     │  │   9   │  vars_menu                   │  alt        │  ...   │     │   │
            │                       │  │                                     │  │  10   │  commands_with_description   │  control    │  ...   │     │   │
            │                       │  │                                     │  ╰───────┴──────────────────────────────┴─────────────┴────────╯     │   │
            │                       │  ╰─────────────────────────────────────┴──────────────────────────────────────────────────────────────────────╯   │
            ╰───────────────────────┴───────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯"#}
    );
    Ok(())
}

#[test]
fn test_collapse_big_0() -> Result {
    Playground::setup("test_expand_big_0", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "sample.toml",
            r#"
            [package]
            authors = ["The Nushell Project Developers"]
            default-run = "nu"
            description = "A new type of shell"
            documentation = "https://www.nushell.sh/book/"
            edition = "2024"
            exclude = ["images"]
            homepage = "https://www.nushell.sh"
            license = "MIT"
            name = "nu"
            repository = "https://github.com/nushell/nushell"
            rust-version = "1.60"
            version = "0.74.1"

            # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

            [package.metadata.binstall]
            pkg-url = "{ repo }/releases/download/{ version }/{ name }-{ version }-{ target }.{ archive-format }"
            pkg-fmt = "tgz"

            [package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
            pkg-fmt = "zip"

            [workspace]
            members = [
                "crates/nu-cli",
                "crates/nu-engine",
                "crates/nu-parser",
                "crates/nu-system",
                "crates/nu-command",
                "crates/nu-protocol",
                "crates/nu-plugin",
                "crates/nu_plugin_inc",
                "crates/nu_plugin_gstat",
                "crates/nu_plugin_example",
                "crates/nu_plugin_query",
                "crates/nu_plugin_custom_values",
                "crates/nu-utils",
            ]

            [dependencies]
            chrono = { version = "0.4.23", features = ["serde"] }
            crossterm = "0.24.0"
            ctrlc = "3.2.1"
            log = "0.4"
            miette = { version = "5.5.0", features = ["fancy-no-backtrace"] }
            nu-ansi-term = "0.46.0"
            nu-cli = { path = "./crates/nu-cli", version = "0.74.1" }
            nu-engine = { path = "./crates/nu-engine", version = "0.74.1" }
            reedline = { version = "0.14.0", features = ["bashisms", "sqlite"] }

            rayon = "1.6.1"
            is_executable = "1.0.1"
            simplelog = "0.12.0"
            time = "0.3.12"

            [target.'cfg(not(target_os = "windows"))'.dependencies]
            # Our dependencies don't use OpenSSL on Windows
            openssl = { version = "0.10.38", features = ["vendored"], optional = true }
            signal-hook = { version = "0.3.14", default-features = false }


            [target.'cfg(windows)'.build-dependencies]
            winres = "0.1"

            [target.'cfg(target_family = "unix")'.dependencies]
            nix = { version = "0.25", default-features = false, features = ["signal", "process", "fs", "term"] }
            atty = "0.2"

            [dev-dependencies]
            nu-test-support = { path = "./crates/nu-test-support", version = "0.74.1" }
            tempfile = "3.2.0"
            assert_cmd = "2.0.2"
            criterion = "0.4"
            pretty_assertions = "1.0.0"
            serial_test = "0.10.0"
            hamcrest2 = "0.3.0"
            rstest = { version = "0.15.0", default-features = false }
            itertools = "0.10.3"

            [features]
            plugin = [
                "nu-plugin",
                "nu-cli/plugin",
                "nu-parser/plugin",
                "nu-command/plugin",
                "nu-protocol/plugin",
                "nu-engine/plugin",
            ]
            # extra used to be more useful but now it's the same as default. Leaving it in for backcompat with existing build scripts
            extra = ["default"]
            default = ["plugin", "which-support", "trash-support", "sqlite"]
            stable = ["default"]
            wasi = []

            # Enable to statically link OpenSSL; otherwise the system version will be used. Not enabled by default because it takes a while to build
            static-link-openssl = ["dep:openssl"]

            # Stable (Default)
            which-support = ["nu-command/which-support"]
            trash-support = ["nu-command/trash-support"]

            # Main nu binary
            [[bin]]
            name = "nu"
            path = "src/main.rs"

            # To use a development version of a dependency please use a global override here
            # changing versions in each sub-crate of the workspace is tedious
            [patch.crates-io]
            reedline = { git = "https://github.com/nushell/reedline.git", branch = "main" }

            # Criterion benchmarking setup
            # Run all benchmarks with `cargo bench`
            # Run individual benchmarks like `cargo bench -- <regex>` e.g. `cargo bench -- parse`
            [[bench]]
            name = "benchmarks"
            harness = false
            "#,
        )]);

        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --width=80 --collapse")?;

        let expected = indoc! {r#"
            ╭──────────────────┬───────────────┬───────────────────────────────────────────╮
            │ package          │ authors       │ The Nushell Project Developers            │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ default-run   │ nu                                        │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ description   │ A new type of shell                       │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ documentation │ https://www.nushell.sh/book/              │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ edition       │ 2024                                      │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ exclude       │ images                                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ homepage      │ https://www.nushell.sh                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ license       │ MIT                                       │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ name          │ nu                                        │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ repository    │ https://github.com/nushell/nushell        │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ rust-version  │ 1.60                                      │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ version       │ 0.74.1                                    │
            │                  ├───────────────┼──────────┬───────────┬────────────────────┤
            │                  │ metadata      │ binstall │ pkg-url   │ { repo }/releases/ │
            │                  │               │          │           │ download/{ v       │
            │                  │               │          │           │ ersion }/{ name }- │
            │                  │               │          │           │ { version }-       │
            │                  │               │          │           │ { target }.{ archi │
            │                  │               │          │           │ ve-format }        │
            │                  │               │          ├───────────┼────────────────────┤
            │                  │               │          │ pkg-fmt   │ tgz                │
            │                  │               │          ├───────────┼────────────────────┤
            │                  │               │          │ overrides │ ...                │
            ├──────────────────┼─────────┬─────┴──────────┴───────────┴────────────────────┤
            │ workspace        │ members │ crates/nu-cli                                   │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-engine                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-parser                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-system                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-command                               │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-protocol                              │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-plugin                                │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_inc                            │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_gstat                          │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_example                        │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_query                          │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_custom_values                  │
            │                  │         ├─────────────────────────────────────────────────┤
            │                  │         │ crates/nu-utils                                 │
            ├──────────────────┼─────────┴─────┬──────────┬────────────────────────────────┤
            │ dependencies     │ chrono        │ version  │ 0.4.23                         │
            │                  │               ├──────────┼────────────────────────────────┤
            │                  │               │ features │ serde                          │
            │                  ├───────────────┼──────────┴────────────────────────────────┤
            │                  │ crossterm     │ 0.24.0                                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ ctrlc         │ 3.2.1                                     │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ log           │ 0.4                                       │
            │                  ├───────────────┼──────────┬────────────────────────────────┤
            │                  │ miette        │ version  │ 5.5.0                          │
            │                  │               ├──────────┼────────────────────────────────┤
            │                  │               │ features │ fancy-no-backtrace             │
            │                  ├───────────────┼──────────┴────────────────────────────────┤
            │                  │ nu-ansi-term  │ 0.46.0                                    │
            │                  ├───────────────┼─────────┬─────────────────────────────────┤
            │                  │ nu-cli        │ path    │ ./crates/nu-cli                 │
            │                  │               ├─────────┼─────────────────────────────────┤
            │                  │               │ version │ 0.74.1                          │
            │                  ├───────────────┼─────────┼─────────────────────────────────┤
            │                  │ nu-engine     │ path    │ ./crates/nu-engine              │
            │                  │               ├─────────┼─────────────────────────────────┤
            │                  │               │ version │ 0.74.1                          │
            │                  ├───────────────┼─────────┴┬────────────────────────────────┤
            │                  │ reedline      │ version  │ 0.14.0                         │
            │                  │               ├──────────┼────────────────────────────────┤
            │                  │               │ features │ bashisms                       │
            │                  │               │          ├────────────────────────────────┤
            │                  │               │          │ sqlite                         │
            │                  ├───────────────┼──────────┴────────────────────────────────┤
            │                  │ rayon         │ 1.6.1                                     │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ is_executable │ 1.0.1                                     │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ simplelog     │ 0.12.0                                    │
            │                  ├───────────────┼───────────────────────────────────────────┤
            │                  │ time          │ 0.3.12                                    │
            ├──────────────────┼───────────────┴─────────────────┬──────────────┬──────────┤
            │ target           │ cfg(not(target_os = "windows")) │ dependencies │ ...      │
            │                  │                                 │              ├──────────┤
            │                  │                                 │              │ ...      │
            │                  ├─────────────────────────────────┼──────────────┴──────────┤
            │                  │ cfg(windows)                    │ ...                     │
            │                  ├─────────────────────────────────┼──────────────┬──────────┤
            │                  │ cfg(target_family = "unix")     │ dependencies │ ...      │
            │                  │                                 │              ├──────────┤
            │                  │                                 │              │ ...      │
            ├──────────────────┼───────────────────┬─────────┬───┴──────────────┴──────────┤
            │ dev-dependencies │ nu-test-support   │ path    │ ./crates/nu-test-support    │
            │                  │                   ├─────────┼─────────────────────────────┤
            │                  │                   │ version │ 0.74.1                      │
            │                  ├───────────────────┼─────────┴─────────────────────────────┤
            │                  │ tempfile          │ 3.2.0                                 │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ assert_cmd        │ 2.0.2                                 │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ criterion         │ 0.4                                   │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ pretty_assertions │ 1.0.0                                 │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ serial_test       │ 0.10.0                                │
            │                  ├───────────────────┼───────────────────────────────────────┤
            │                  │ hamcrest2         │ 0.3.0                                 │
            │                  ├───────────────────┼──────────────────┬────────────────────┤
            │                  │ rstest            │ version          │ 0.15.0             │
            │                  │                   ├──────────────────┼────────────────────┤
            │                  │                   │ default-features │ false              │
            │                  ├───────────────────┼──────────────────┴────────────────────┤
            │                  │ itertools         │ 0.10.3                                │
            ├──────────────────┼───────────────────┴─┬─────────────────────────────────────┤
            │ features         │ plugin              │ nu-plugin                           │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-cli/plugin                       │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-parser/plugin                    │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-command/plugin                   │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-protocol/plugin                  │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ nu-engine/plugin                    │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ extra               │ default                             │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ default             │ plugin                              │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ which-support                       │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ trash-support                       │
            │                  │                     ├─────────────────────────────────────┤
            │                  │                     │ sqlite                              │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ stable              │ default                             │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ wasi                │                                     │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ static-link-openssl │ dep:openssl                         │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ which-support       │ nu-command/which-support            │
            │                  ├─────────────────────┼─────────────────────────────────────┤
            │                  │ trash-support       │ nu-command/trash-support            │
            ├──────────────────┼──────┬──────────────┴─────────────────────────────────────┤
            │ bin              │ name │ path                                               │
            │                  ├──────┼────────────────────────────────────────────────────┤
            │                  │ nu   │ src/main.rs                                        │
            ├──────────────────┼──────┴────┬──────────┬────────┬───────────────────────────┤
            │ patch            │ crates-io │ reedline │ git    │ https://github.com/nushel │
            │                  │           │          │        │ l/reedline.git            │
            │                  │           │          ├────────┼───────────────────────────┤
            │                  │           │          │ branch │ main                      │
            ├──────────────────┼───────────┴┬─────────┴────────┴───────────────────────────┤
            │ bench            │ name       │ harness                                      │
            │                  ├────────────┼──────────────────────────────────────────────┤
            │                  │ benchmarks │ false                                        │
            ╰──────────────────┴────────────┴──────────────────────────────────────────────╯"#};

        assert_eq!(actual, expected);

        let actual: String = test()
            .cwd(dirs.test())
            .run("open sample.toml | table --collapse --width=160")?;

        let expected = indoc! {r#"
            ╭──────────────────┬───────────────┬──────────────────────────────────────────────────────────────────────────╮
            │ package          │ authors       │ The Nushell Project Developers                                           │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ default-run   │ nu                                                                       │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ description   │ A new type of shell                                                      │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ documentation │ https://www.nushell.sh/book/                                             │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ edition       │ 2024                                                                     │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ exclude       │ images                                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ homepage      │ https://www.nushell.sh                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ license       │ MIT                                                                      │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ name          │ nu                                                                       │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ repository    │ https://github.com/nushell/nushell                                       │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ rust-version  │ 1.60                                                                     │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ version       │ 0.74.1                                                                   │
            │                  ├───────────────┼──────────┬───────────┬───────────────────────────────────────────────────┤
            │                  │ metadata      │ binstall │ pkg-url   │ { repo }/releases/download/{ v                    │
            │                  │               │          │           │ ersion }/{ name }-{ version }-                    │
            │                  │               │          │           │ { target }.{ archive-format }                     │
            │                  │               │          ├───────────┼───────────────────────────────────────────────────┤
            │                  │               │          │ pkg-fmt   │ tgz                                               │
            │                  │               │          ├───────────┼────────────────────────┬─────────┬────────────────┤
            │                  │               │          │ overrides │ x86_64-pc-windows-msvc │ pkg-fmt │ zip            │
            ├──────────────────┼─────────┬─────┴──────────┴───────────┴────────────────────────┴─────────┴────────────────┤
            │ workspace        │ members │ crates/nu-cli                                                                  │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-engine                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-parser                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-system                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-command                                                              │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-protocol                                                             │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-plugin                                                               │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_inc                                                           │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_gstat                                                         │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_example                                                       │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_query                                                         │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu_plugin_custom_values                                                 │
            │                  │         ├────────────────────────────────────────────────────────────────────────────────┤
            │                  │         │ crates/nu-utils                                                                │
            ├──────────────────┼─────────┴─────┬──────────┬───────────────────────────────────────────────────────────────┤
            │ dependencies     │ chrono        │ version  │ 0.4.23                                                        │
            │                  │               ├──────────┼───────────────────────────────────────────────────────────────┤
            │                  │               │ features │ serde                                                         │
            │                  ├───────────────┼──────────┴───────────────────────────────────────────────────────────────┤
            │                  │ crossterm     │ 0.24.0                                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ ctrlc         │ 3.2.1                                                                    │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ log           │ 0.4                                                                      │
            │                  ├───────────────┼──────────┬───────────────────────────────────────────────────────────────┤
            │                  │ miette        │ version  │ 5.5.0                                                         │
            │                  │               ├──────────┼───────────────────────────────────────────────────────────────┤
            │                  │               │ features │ fancy-no-backtrace                                            │
            │                  ├───────────────┼──────────┴───────────────────────────────────────────────────────────────┤
            │                  │ nu-ansi-term  │ 0.46.0                                                                   │
            │                  ├───────────────┼─────────┬────────────────────────────────────────────────────────────────┤
            │                  │ nu-cli        │ path    │ ./crates/nu-cli                                                │
            │                  │               ├─────────┼────────────────────────────────────────────────────────────────┤
            │                  │               │ version │ 0.74.1                                                         │
            │                  ├───────────────┼─────────┼────────────────────────────────────────────────────────────────┤
            │                  │ nu-engine     │ path    │ ./crates/nu-engine                                             │
            │                  │               ├─────────┼────────────────────────────────────────────────────────────────┤
            │                  │               │ version │ 0.74.1                                                         │
            │                  ├───────────────┼─────────┴┬───────────────────────────────────────────────────────────────┤
            │                  │ reedline      │ version  │ 0.14.0                                                        │
            │                  │               ├──────────┼───────────────────────────────────────────────────────────────┤
            │                  │               │ features │ bashisms                                                      │
            │                  │               │          ├───────────────────────────────────────────────────────────────┤
            │                  │               │          │ sqlite                                                        │
            │                  ├───────────────┼──────────┴───────────────────────────────────────────────────────────────┤
            │                  │ rayon         │ 1.6.1                                                                    │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ is_executable │ 1.0.1                                                                    │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ simplelog     │ 0.12.0                                                                   │
            │                  ├───────────────┼──────────────────────────────────────────────────────────────────────────┤
            │                  │ time          │ 0.3.12                                                                   │
            ├──────────────────┼───────────────┴─────────────────┬──────────────┬─────────────┬──────────┬────────────────┤
            │ target           │ cfg(not(target_os = "windows")) │ dependencies │ openssl     │ version  │ 0.10.38        │
            │                  │                                 │              │             ├──────────┼────────────────┤
            │                  │                                 │              │             │ features │ vendored       │
            │                  │                                 │              │             ├──────────┼────────────────┤
            │                  │                                 │              │             │ optional │ true           │
            │                  │                                 │              ├─────────────┼──────────┴───────┬────────┤
            │                  │                                 │              │ signal-hook │ version          │ 0.3.14 │
            │                  │                                 │              │             ├──────────────────┼────────┤
            │                  │                                 │              │             │ default-features │ false  │
            │                  ├─────────────────────────────────┼──────────────┴─────┬───────┴┬─────────────────┴────────┤
            │                  │ cfg(windows)                    │ build-dependencies │ winres │ 0.1                      │
            │                  ├─────────────────────────────────┼──────────────┬─────┴┬───────┴──────────┬───────────────┤
            │                  │ cfg(target_family = "unix")     │ dependencies │ nix  │ version          │ 0.25          │
            │                  │                                 │              │      ├──────────────────┼───────────────┤
            │                  │                                 │              │      │ default-features │ false         │
            │                  │                                 │              │      ├──────────────────┼───────────────┤
            │                  │                                 │              │      │ features         │ signal        │
            │                  │                                 │              │      │                  ├───────────────┤
            │                  │                                 │              │      │                  │ process       │
            │                  │                                 │              │      │                  ├───────────────┤
            │                  │                                 │              │      │                  │ fs            │
            │                  │                                 │              │      │                  ├───────────────┤
            │                  │                                 │              │      │                  │ term          │
            │                  │                                 │              ├──────┼──────────────────┴───────────────┤
            │                  │                                 │              │ atty │ 0.2                              │
            ├──────────────────┼───────────────────┬─────────┬───┴──────────────┴──────┴──────────────────────────────────┤
            │ dev-dependencies │ nu-test-support   │ path    │ ./crates/nu-test-support                                   │
            │                  │                   ├─────────┼────────────────────────────────────────────────────────────┤
            │                  │                   │ version │ 0.74.1                                                     │
            │                  ├───────────────────┼─────────┴────────────────────────────────────────────────────────────┤
            │                  │ tempfile          │ 3.2.0                                                                │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ assert_cmd        │ 2.0.2                                                                │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ criterion         │ 0.4                                                                  │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ pretty_assertions │ 1.0.0                                                                │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ serial_test       │ 0.10.0                                                               │
            │                  ├───────────────────┼──────────────────────────────────────────────────────────────────────┤
            │                  │ hamcrest2         │ 0.3.0                                                                │
            │                  ├───────────────────┼──────────────────┬───────────────────────────────────────────────────┤
            │                  │ rstest            │ version          │ 0.15.0                                            │
            │                  │                   ├──────────────────┼───────────────────────────────────────────────────┤
            │                  │                   │ default-features │ false                                             │
            │                  ├───────────────────┼──────────────────┴───────────────────────────────────────────────────┤
            │                  │ itertools         │ 0.10.3                                                               │
            ├──────────────────┼───────────────────┴─┬────────────────────────────────────────────────────────────────────┤
            │ features         │ plugin              │ nu-plugin                                                          │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-cli/plugin                                                      │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-parser/plugin                                                   │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-command/plugin                                                  │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-protocol/plugin                                                 │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ nu-engine/plugin                                                   │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ extra               │ default                                                            │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ default             │ plugin                                                             │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ which-support                                                      │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ trash-support                                                      │
            │                  │                     ├────────────────────────────────────────────────────────────────────┤
            │                  │                     │ sqlite                                                             │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ stable              │ default                                                            │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ wasi                │                                                                    │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ static-link-openssl │ dep:openssl                                                        │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ which-support       │ nu-command/which-support                                           │
            │                  ├─────────────────────┼────────────────────────────────────────────────────────────────────┤
            │                  │ trash-support       │ nu-command/trash-support                                           │
            ├──────────────────┼──────┬──────────────┴────────────────────────────────────────────────────────────────────┤
            │ bin              │ name │ path                                                                              │
            │                  ├──────┼───────────────────────────────────────────────────────────────────────────────────┤
            │                  │ nu   │ src/main.rs                                                                       │
            ├──────────────────┼──────┴────┬──────────┬────────┬──────────────────────────────────────────────────────────┤
            │ patch            │ crates-io │ reedline │ git    │ https://github.com/nushell/reedline.git                  │
            │                  │           │          ├────────┼──────────────────────────────────────────────────────────┤
            │                  │           │          │ branch │ main                                                     │
            ├──────────────────┼───────────┴┬─────────┴────────┴──────────────────────────────────────────────────────────┤
            │ bench            │ name       │ harness                                                                     │
            │                  ├────────────┼─────────────────────────────────────────────────────────────────────────────┤
            │                  │ benchmarks │ false                                                                       │
            ╰──────────────────┴────────────┴─────────────────────────────────────────────────────────────────────────────╯"#};

        assert_eq!(actual, expected);
        Ok(())
    })
}

#[test]
fn table_expand_index_offset() -> Result {
    let actual: String = test().run("1..1002 | table --width=80 --expand")?;
    let suffix = indoc! {"
        ╭──────┬──────╮
        │ 1000 │ 1001 │
        │ 1001 │ 1002 │
        ╰──────┴──────╯
    "};
    let expected_suffix = actual.strip_suffix(suffix);
    assert!(expected_suffix.is_some(), "{actual:?}");
    Ok(())
}

#[test]
fn table_index_offset() -> Result {
    let actual: String = test().run("1..1002 | table --width=80")?;
    let suffix = indoc! {"
        ╭──────┬──────╮
        │ 1000 │ 1001 │
        │ 1001 │ 1002 │
        ╰──────┴──────╯
    "};
    let expected_suffix = actual.strip_suffix(suffix);
    assert!(expected_suffix.is_some(), "{actual:?}");
    Ok(())
}

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
fn table_padding_not_default() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.padding = 5
                $data | table --width=80
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───────────┬───────────┬───────────┬────────────────────────╮
            │     #     │     a     │     b     │           c            │
            ├───────────┼───────────┼───────────┼────────────────────────┤
            │     0     │     1     │     2     │                  3     │
            │     1     │     4     │     5     │     [list 3 items]     │
            ╰───────────┴───────────┴───────────┴────────────────────────╯
        "})
}

#[test]
fn table_padding_zero() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.padding = {left: 0, right: 0}
                $data | table --width=80
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭─┬─┬─┬──────────────╮
            │#│a│b│      c       │
            ├─┼─┼─┼──────────────┤
            │0│1│2│             3│
            │1│4│5│[list 3 items]│
            ╰─┴─┴─┴──────────────╯
        "})
}

#[test]
fn table_expand_padding_not_default() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.padding = 5
                $data | table --width=80 -e
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭─────────────┬─────────────┬─────────────┬────────────────────────────────────╮
            │       #     │      a      │      b      │                 c                  │
            ├─────────────┼─────────────┼─────────────┼────────────────────────────────────┤
            │       0     │       1     │       2     │                              3     │
            │       1     │       4     │       5     │     ╭───────────┬───────────╮      │
            │             │             │             │     │     0     │     1     │      │
            │             │             │             │     │     1     │     2     │      │
            │             │             │             │     │     2     │     3     │      │
            │             │             │             │     ╰───────────┴───────────╯      │
            ╰─────────────┴─────────────┴─────────────┴────────────────────────────────────╯
        "})
}

#[test]
fn table_expand_padding_zero() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.padding = {left: 0, right: 0}
                $data | table --width=80 -e
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭─┬─┬─┬─────╮
            │#│a│b│  c  │
            ├─┼─┼─┼─────┤
            │0│1│2│    3│
            │1│4│5│╭─┬─╮│
            │ │ │ ││0│1││
            │ │ │ ││1│2││
            │ │ │ ││2│3││
            │ │ │ │╰─┴─╯│
            ╰─┴─┴─┴─────╯
        "})
}

#[test]
fn table_collapse_padding_not_default() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.padding = 5
                $data | table --width=80 -c
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───────────┬───────────┬───────────╮
            │     a     │     b     │     c     │
            ├───────────┼───────────┼───────────┤
            │     1     │     2     │     3     │
            ├───────────┼───────────┼───────────┤
            │     4     │     5     │     1     │
            │           │           ├───────────┤
            │           │           │     2     │
            │           │           ├───────────┤
            │           │           │     3     │
            ╰───────────┴───────────┴───────────╯
        "})
}

#[test]
fn table_collapse_padding_zero() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.padding = {left: 0, right: 0}
                $data | table --width=80 -c
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭─┬─┬─╮
            │a│b│c│
            ├─┼─┼─┤
            │1│2│3│
            ├─┼─┼─┤
            │4│5│1│
            │ │ ├─┤
            │ │ │2│
            │ │ ├─┤
            │ │ │3│
            ╰─┴─┴─╯
        "})
}

#[test]
fn table_leading_trailing_space_bg() -> Result {
    test()
        .run_with_data(
            r#"
                let data = $in
                $env.config.color_config.leading_trailing_space_bg = { bg: 'default' }
                $data
                | table --width=80
            "#,
            test_value!([
                { a: "  1  ", b: "    2", "c   ": "3    " },
                { a: "  4  ", b: "hello\nworld", "c   ": ["  1  ", 2, [1, "  2  ", 3]] }
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───────┬───────┬────────────────╮
            │ # │   a   │   b   │      c         │
            ├───┼───────┼───────┼────────────────┤
            │ 0 │   1   │     2 │ 3              │
            │ 1 │   4   │ hello │ [list 3 items] │
            │   │       │ world │                │
            ╰───┴───────┴───────┴────────────────╯
        "})
}

#[test]
fn table_leading_trailing_space_bg_expand() -> Result {
    test()
        .run_with_data(
            r#"
                let data = $in
                $env.config.color_config.leading_trailing_space_bg = { bg: 'default' }
                $data
                | table --width=80 --expand
            "#,
            test_value!([
                { a: "  1  ", b: "    2", "c   ": "3    " },
                { a: "  4  ", b: "hello\nworld", "c   ": ["  1  ", 2, [1, "  2  ", 3]] }
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───────┬───────┬───────────────────────╮
            │ # │   a   │   b   │         c             │
            ├───┼───────┼───────┼───────────────────────┤
            │ 0 │   1   │     2 │ 3                     │
            │ 1 │   4   │ hello │ ╭───┬───────────────╮ │
            │   │       │ world │ │ 0 │   1           │ │
            │   │       │       │ │ 1 │             2 │ │
            │   │       │       │ │ 2 │ ╭───┬───────╮ │ │
            │   │       │       │ │   │ │ 0 │     1 │ │ │
            │   │       │       │ │   │ │ 1 │   2   │ │ │
            │   │       │       │ │   │ │ 2 │     3 │ │ │
            │   │       │       │ │   │ ╰───┴───────╯ │ │
            │   │       │       │ ╰───┴───────────────╯ │
            ╰───┴───────┴───────┴───────────────────────╯
        "})
}

#[rstest]
#[case::all_rows(
    "table --width=80 -a 100",
    indoc! {"
        ╭───┬───┬───┬────────────────╮
        │ # │ a │ b │       c        │
        ├───┼───┼───┼────────────────┤
        │ 0 │ 1 │ 2 │              3 │
        │ 1 │ 4 │ 5 │ [list 3 items] │
        │ 2 │ 1 │ 2 │              3 │
        │ 3 │ 1 │ 2 │              3 │
        │ 4 │ 1 │ 2 │              3 │
        │ 5 │ 1 │ 2 │              3 │
        │ 6 │ 1 │ 2 │              3 │
        ╰───┴───┴───┴────────────────╯
    "}
)]
#[case::two_rows(
    "table --width=80 -a 2",
    indoc! {"
        ╭───┬─────┬─────┬────────────────╮
        │ # │  a  │  b  │       c        │
        ├───┼─────┼─────┼────────────────┤
        │ 0 │   1 │   2 │              3 │
        │ 1 │   4 │   5 │ [list 3 items] │
        │ 2 │ ... │ ... │ ...            │
        │ 3 │   1 │   2 │              3 │
        │ 4 │   1 │   2 │              3 │
        ╰───┴─────┴─────┴────────────────╯
    "}
)]
#[case::one_row(
    "table --width=80 -a 1",
    indoc! {"
        ╭───┬─────┬─────┬─────╮
        │ # │  a  │  b  │  c  │
        ├───┼─────┼─────┼─────┤
        │ 0 │   1 │   2 │   3 │
        │ 1 │ ... │ ... │ ... │
        │ 2 │   1 │   2 │   3 │
        ╰───┴─────┴─────┴─────╯
    "}
)]
#[case::zero_rows("table --width=80 -a 0", "")]
fn table_abbreviation(#[case] command: &str, #[case] expected: &str) -> Result {
    test()
        .run_with_data(
            command,
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(expected)
}

#[test]
fn table_abbreviation_kv() -> Result {
    let mut tester = test();
    let input: Value = test_value!({
        a: 1,
        b: {
            a: 1,
            b: [1, 2, 3],
            c: [1, 2, 3],
        },
        c: [1, 2, [1, 2, 3], 3],
        e: 1,
        q: 2,
        t: 4,
        r: 1,
        x: 9,
    });

    tester
        .run_with_data("table --width=80 -a 100", input.clone())
        .expect_value_eq(indoc! {"
            ╭───┬───────────────────╮
            │ a │ 1                 │
            │ b │ {record 3 fields} │
            │ c │ [list 4 items]    │
            │ e │ 1                 │
            │ q │ 2                 │
            │ t │ 4                 │
            │ r │ 1                 │
            │ x │ 9                 │
            ╰───┴───────────────────╯"})?;

    tester
        .run_with_data("table --width=80 -a 2", input.clone())
        .expect_value_eq(indoc! {"
            ╭─────┬───────────────────╮
            │ a   │ 1                 │
            │ b   │ {record 3 fields} │
            │ ... │ ...               │
            │ r   │ 1                 │
            │ x   │ 9                 │
            ╰─────┴───────────────────╯"})?;

    tester
        .run_with_data("table --width=80 -a 1", input.clone())
        .expect_value_eq(indoc! {"
            ╭─────┬─────╮
            │ a   │ 1   │
            │ ... │ ... │
            │ x   │ 9   │
            ╰─────┴─────╯"})?;

    tester
        .run_with_data("table --width=80 -a 0", input)
        .expect_value_eq(indoc! {"
            ╭─────┬─────╮
            │ ... │ ... │
            ╰─────┴─────╯"})
}

#[test]
fn table_abbreviation_kv_expand() -> Result {
    let mut tester = test();
    let input: Value = test_value!({
        a: 1,
        b: {
            a: 1,
            b: [1, 2, 3],
            c: [1, 2, 3],
        },
        c: [1, 2, [1, 2, 3], 3],
        e: 1,
        q: 2,
        t: 4,
        r: 1,
        x: 9,
    });

    tester
        .run_with_data("table --width=80 -a 100 -e", input.clone())
        .expect_value_eq(indoc! {"
            ╭───┬───────────────────╮
            │ a │ 1                 │
            │   │ ╭───┬───────────╮ │
            │ b │ │ a │ 1         │ │
            │   │ │   │ ╭───┬───╮ │ │
            │   │ │ b │ │ 0 │ 1 │ │ │
            │   │ │   │ │ 1 │ 2 │ │ │
            │   │ │   │ │ 2 │ 3 │ │ │
            │   │ │   │ ╰───┴───╯ │ │
            │   │ │   │ ╭───┬───╮ │ │
            │   │ │ c │ │ 0 │ 1 │ │ │
            │   │ │   │ │ 1 │ 2 │ │ │
            │   │ │   │ │ 2 │ 3 │ │ │
            │   │ │   │ ╰───┴───╯ │ │
            │   │ ╰───┴───────────╯ │
            │   │ ╭───┬───────────╮ │
            │ c │ │ 0 │         1 │ │
            │   │ │ 1 │         2 │ │
            │   │ │ 2 │ ╭───┬───╮ │ │
            │   │ │   │ │ 0 │ 1 │ │ │
            │   │ │   │ │ 1 │ 2 │ │ │
            │   │ │   │ │ 2 │ 3 │ │ │
            │   │ │   │ ╰───┴───╯ │ │
            │   │ │ 3 │         3 │ │
            │   │ ╰───┴───────────╯ │
            │ e │ 1                 │
            │ q │ 2                 │
            │ t │ 4                 │
            │ r │ 1                 │
            │ x │ 9                 │
            ╰───┴───────────────────╯"})?;

    tester
        .run_with_data("table --width=80 -a 2 -e", input.clone())
        .expect_value_eq(indoc! {"
            ╭─────┬───────────────────╮
            │ a   │ 1                 │
            │     │ ╭───┬───────────╮ │
            │ b   │ │ a │ 1         │ │
            │     │ │   │ ╭───┬───╮ │ │
            │     │ │ b │ │ 0 │ 1 │ │ │
            │     │ │   │ │ 1 │ 2 │ │ │
            │     │ │   │ │ 2 │ 3 │ │ │
            │     │ │   │ ╰───┴───╯ │ │
            │     │ │   │ ╭───┬───╮ │ │
            │     │ │ c │ │ 0 │ 1 │ │ │
            │     │ │   │ │ 1 │ 2 │ │ │
            │     │ │   │ │ 2 │ 3 │ │ │
            │     │ │   │ ╰───┴───╯ │ │
            │     │ ╰───┴───────────╯ │
            │ ... │ ...               │
            │ r   │ 1                 │
            │ x   │ 9                 │
            ╰─────┴───────────────────╯"})?;

    tester
        .run_with_data("table --width=80 -a 1 -e", input.clone())
        .expect_value_eq(indoc! {"
            ╭─────┬─────╮
            │ a   │ 1   │
            │ ... │ ... │
            │ x   │ 9   │
            ╰─────┴─────╯"})?;

    tester
        .run_with_data("table --width=80 -a 0 -e", input)
        .expect_value_eq(indoc! {"
            ╭─────┬─────╮
            │ ... │ ... │
            ╰─────┴─────╯"})
}

#[test]
fn table_abbreviation_by_config() -> Result {
    let mut tester = test();

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.table.abbreviated_row_count = 100
                $data | table --width=80
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬───┬───┬────────────────╮
            │ # │ a │ b │       c        │
            ├───┼───┼───┼────────────────┤
            │ 0 │ 1 │ 2 │              3 │
            │ 1 │ 4 │ 5 │ [list 3 items] │
            │ 2 │ 1 │ 2 │              3 │
            │ 3 │ 1 │ 2 │              3 │
            │ 4 │ 1 │ 2 │              3 │
            │ 5 │ 1 │ 2 │              3 │
            │ 6 │ 1 │ 2 │              3 │
            ╰───┴───┴───┴────────────────╯
        "})?;

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.table.abbreviated_row_count = 2
                $data | table --width=80
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬─────┬─────┬────────────────╮
            │ # │  a  │  b  │       c        │
            ├───┼─────┼─────┼────────────────┤
            │ 0 │   1 │   2 │              3 │
            │ 1 │   4 │   5 │ [list 3 items] │
            │ 2 │ ... │ ... │ ...            │
            │ 3 │   1 │   2 │              3 │
            │ 4 │   1 │   2 │              3 │
            ╰───┴─────┴─────┴────────────────╯
        "})?;

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.table.abbreviated_row_count = 1
                $data | table --width=80
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬─────┬─────┬─────╮
            │ # │  a  │  b  │  c  │
            ├───┼─────┼─────┼─────┤
            │ 0 │   1 │   2 │   3 │
            │ 1 │ ... │ ... │ ... │
            │ 2 │   1 │   2 │   3 │
            ╰───┴─────┴─────┴─────╯
        "})?;

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.table.abbreviated_row_count = 0
                $data | table --width=80
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
            ],
        )
        .expect_value_eq("")
}

#[test]
fn table_abbreviation_by_config_override() -> Result {
    let mut tester = test();

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.table.abbreviated_row_count = 2
                $data | table --width=80 -a 1
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬─────┬─────┬─────╮
            │ # │  a  │  b  │  c  │
            ├───┼─────┼─────┼─────┤
            │ 0 │   1 │   2 │   3 │
            │ 1 │ ... │ ... │ ... │
            │ 2 │   1 │   2 │   3 │
            ╰───┴─────┴─────┴─────╯
        "})?;

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.table.abbreviated_row_count = 1
                $data | table --width=80 -a 2
            ",
            test_table![
                ["a", "b", "c"];
                [1, 2, 3],
                [4, 5, [1, 2, 3]],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
                [1, 2, 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭───┬─────┬─────┬────────────────╮
            │ # │  a  │  b  │       c        │
            ├───┼─────┼─────┼────────────────┤
            │ 0 │   1 │   2 │              3 │
            │ 1 │   4 │   5 │ [list 3 items] │
            │ 2 │ ... │ ... │ ...            │
            │ 3 │   1 │   2 │              3 │
            │ 4 │   1 │   2 │              3 │
            ╰───┴─────┴─────┴────────────────╯
        "})
}

#[test]
fn table_abbreviation_cut() -> Result {
    let actual: String = test().run("0..2000 | table --width=80 -a 0")?;
    assert_eq!(actual, "");

    let actual: String = test().run("0..2000 | table --width=80 -a 1")?;
    assert_eq!(
        actual,
        indoc! {"
            ╭───┬──────╮
            │ 0 │    0 │
            │ 1 │ ...  │
            │ 2 │ 2000 │
            ╰───┴──────╯
        "}
    );

    let actual: String = test().run("0..2000 | table --width=80 -a 3")?;
    assert_eq!(
        actual,
        indoc! {"
            ╭───┬──────╮
            │ 0 │    0 │
            │ 1 │    1 │
            │ 2 │    2 │
            │ 3 │ ...  │
            │ 4 │ 1998 │
            │ 5 │ 1999 │
            │ 6 │ 2000 │
            ╰───┴──────╯
        "}
    );

    let rows = (0..=2000)
        .map(|i| format!("│{i:>5} │{i:>5} │"))
        .collect::<Vec<_>>()
        .join("\n");
    let output = format!("{}\n{}\n{}\n", "╭──────┬──────╮", rows, "╰──────┴──────╯");

    let actual: String = test().run("0..2000 | table --width=80 -a 2000")?;
    assert_eq!(actual, output);

    let actual: String = test().run("0..2000 | table --width=80 -a 200000")?;
    assert_eq!(actual, output);
    Ok(())
}
#[test]
fn table_expand_inner_index_0() -> Result {
    let actual: String = test().run("0..1000 | each { [0] } | table -e")?;
    let rows = (0..1000)
        .flat_map(|i| {
            [
                format!("│{i:>4} │ ╭───┬───╮ │"),
                "│     │ │ 0 │ 0 │ │".to_string(),
                "│     │ ╰───┴───╯ │".to_string(),
            ]
        })
        .collect::<Vec<_>>()
        .join("\n");
    let expected = format!(
        "{}\n{}\n{}",
        "╭─────┬───────────╮",
        rows,
        indoc! {"
            ╰─────┴───────────╯
            ╭──────┬───────────╮
            │ 1000 │ ╭───┬───╮ │
            │      │ │ 0 │ 0 │ │
            │      │ ╰───┴───╯ │
            ╰──────┴───────────╯
        "}
    );
    assert_eq!(actual, expected);
    Ok(())
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

#[rstest]
#[case::no_index(
    "table --width=80 --theme basic -i false",
    indoc! {"
        +---+----------------+
        | a |       b        |
        +---+----------------+
        | 1 |              2 |
        +---+----------------+
        | 2 | [list 2 items] |
        +---+----------------+
    "}
)]
#[case::with_index(
    "table --width=80 --theme basic -i true",
    indoc! {"
        +---+---+----------------+
        | # | a |       b        |
        +---+---+----------------+
        | 0 | 1 |              2 |
        +---+---+----------------+
        | 1 | 2 | [list 2 items] |
        +---+---+----------------+
    "}
)]
#[case::offset_index(
    "table --width=80 --theme basic -i 10",
    indoc! {"
        +----+---+----------------+
        |  # | a |       b        |
        +----+---+----------------+
        | 10 | 1 |              2 |
        +----+---+----------------+
        | 11 | 2 | [list 2 items] |
        +----+---+----------------+
    "}
)]
fn table_index_arg(#[case] command: &str, #[case] expected: &str) -> Result {
    test()
        .run_with_data(
            command,
            test_table![
                ["a", "b"];
                [1, 2],
                [2, [4, 4]],
            ],
        )
        .expect_value_eq(expected)
}

#[rstest]
#[case::no_index(
    "table --width=80 --theme basic --expand -i false",
    indoc! {"
        +---+-------+
        | a |   b   |
        +---+-------+
        | 1 |     2 |
        +---+-------+
        | 2 | +---+ |
        |   | | 4 | |
        |   | +---+ |
        |   | | 4 | |
        |   | +---+ |
        +---+-------+
    "}
)]
#[case::with_index(
    "table --width=80 --theme basic --expand -i true",
    indoc! {"
        +---+---+-----------+
        | # | a |     b     |
        +---+---+-----------+
        | 0 | 1 |         2 |
        +---+---+-----------+
        | 1 | 2 | +---+---+ |
        |   |   | | 0 | 4 | |
        |   |   | +---+---+ |
        |   |   | | 1 | 4 | |
        |   |   | +---+---+ |
        +---+---+-----------+
    "}
)]
#[case::offset_index(
    "table --width=80 --theme basic --expand -i 10",
    indoc! {"
        +----+---+-----------+
        |  # | a |     b     |
        +----+---+-----------+
        | 10 | 1 |         2 |
        +----+---+-----------+
        | 11 | 2 | +---+---+ |
        |    |   | | 0 | 4 | |
        |    |   | +---+---+ |
        |    |   | | 1 | 4 | |
        |    |   | +---+---+ |
        +----+---+-----------+
    "}
)]
fn table_expand_index_arg(#[case] command: &str, #[case] expected: &str) -> Result {
    test()
        .run_with_data(
            command,
            test_table![
                ["a", "b"];
                [1, 2],
                [2, [4, 4]],
            ],
        )
        .expect_value_eq(expected)
}

#[test]
fn table_list() -> Result {
    let expected = indoc! {"
        ╭────┬────────────────╮
        │  0 │ basic          │
        │  1 │ compact        │
        │  2 │ compact_double │
        │  3 │ default        │
        │  4 │ frameless      │
        │  5 │ heavy          │
        │  6 │ light          │
        │  7 │ none           │
        │  8 │ reinforced     │
        │  9 │ rounded        │
        │ 10 │ thin           │
        │ 11 │ with_love      │
        │ 12 │ psql           │
        │ 13 │ markdown       │
        │ 14 │ dots           │
        │ 15 │ restructured   │
        │ 16 │ ascii_rounded  │
        │ 17 │ basic_compact  │
        │ 18 │ single         │
        │ 19 │ double         │
        ╰────┴────────────────╯
    "};
    let mut tester = test();

    tester
        .run("table --list | table")
        .expect_value_eq(expected)?;
    tester
        .run("ls | table --list | table")
        .expect_value_eq(expected)?;
    tester
        .run("table --list --theme basic | table")
        .expect_value_eq(expected)
}

#[test]
fn table_kv_header_on_separator_trim_algorithm() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.header_on_separator = true
                $data | table --width=60 --theme basic
            ",
            test_record! {
                "key1" => "111111111111111111111111111111111111111111111111111111111111",
            },
        )
        .expect_value_eq(indoc! {"
            +------+---------------------------------------------------+
            | key1 | 1111111111111111111111111111111111111111111111111 |
            |      | 11111111111                                       |
            +------+---------------------------------------------------+"})
}

#[test]
fn table_general_header_on_separator_trim_algorithm() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.header_on_separator = true
                $data | table --width=20 --theme basic
            ",
            test_table![
                ["a", "b"];
                ["11111111111111111111111111111111111111", 2],
            ],
        )
        .expect_value_eq(indoc! {"
            +-#-+----a-----+-b-+
            | 0 | 11111111 | 2 |
            |   | 11111111 |   |
            |   | 11111111 |   |
            |   | 11111111 |   |
            |   | 111111   |   |
            +---+----------+---+
        "})
}

#[test]
fn table_general_header_on_separator_issue1() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.header_on_separator = true
                $data | table --width=87 --theme basic
            ",
            test_table![
                [
                    "Llll oo Bbbbbbbb",
                    "Bbbbbbbb Aaaa",
                    "Nnnnnn",
                    "Ggggg",
                    "Xxxxx Llllllll #",
                    "Bbb",
                    "Pppp Ccccc",
                    "Rrrrrrrr Dddd",
                    "Rrrrrr",
                    "Rrrrrr Ccccc II",
                    "Rrrrrr Ccccc Ppppppp II",
                    "Pppppp Dddddddd Tttt",
                    "Pppppp Dddddddd Dddd",
                    "Rrrrrrrrr Trrrrrr",
                    "Pppppp Ppppp Dddd",
                    "Ppppp Dddd",
                    "Hhhh",
                ];
                [
                    "RRRRRRR",
                    "FFFFFFFF",
                    "UUUU",
                    "VV",
                    202407160001i64,
                    "BBB",
                    1,
                    "7/16/2024",
                    "",
                    "AAA-1111",
                    "AAA-1111-11",
                    "7 YEARS",
                    2555,
                    "RRRRRRRR DDDD",
                    "7/16/2031",
                    "7/16/2031",
                    "NN",
                ],
            ],
        )
        .expect_value_eq(indoc! {"
            +-#-+-Llll oo Bbbbbbbb-+-Bbbbbbbb Aaaa-+-Nnnnnn-+-Ggggg-+-Xxxxx Llllllll #-+-...-+
            | 0 | RRRRRRR          | FFFFFFFF      | UUUU   | VV    |     202407160001 | ... |
            +---+------------------+---------------+--------+-------+------------------+-----+
        "})
}

#[test]
fn table_footer_inheritance() -> Result {
    let field0 = test_table![
        ["y1", "y2", "y3"];
        [1, 2, 3],
        [79, 79, 79],
        [test_value!({ f1: "a string", f2: 1000 }), 1, 2],
    ];
    let field3 = Value::test_list(
        (0..212)
            .map(|_| test_record! { "head1" => 79, "head2" => 79, "head3" => 79 })
            .collect(),
    );
    let field5 = test_table![
        ["x1", "x2", "x3"];
        [1, 2, 3],
        [79, 79, 79],
        [test_value!({ f1: "a string", f2: 1000 }), 1, 2],
    ];

    let actual: String = test().run_with_data(
        "
            let table = $in
            $env.config.table.footer_inheritance = true
            $table
            | table --width=80 --expand
        ",
        test_value!({
            field0: (field0),
            field1: ["a", "b", "c"],
            field2: [123, 234, 345],
            field3: (field3),
            field4: { f1: 1, f2: 3, f3: { f1: "f1", f2: "f2", f3: "f3" } },
            field5: (field5),
        }),
    )?;

    assert_eq!(actual.match_indices("head1").count(), 2);
    assert_eq!(actual.match_indices("head2").count(), 2);
    assert_eq!(actual.match_indices("head3").count(), 2);
    assert_eq!(actual.match_indices("y1").count(), 1);
    assert_eq!(actual.match_indices("y2").count(), 1);
    assert_eq!(actual.match_indices("y3").count(), 1);
    assert_eq!(actual.match_indices("x1").count(), 1);
    assert_eq!(actual.match_indices("x2").count(), 1);
    assert_eq!(actual.match_indices("x3").count(), 1);
    Ok(())
}

#[test]
fn table_footer_inheritance_kv_rows() -> Result {
    let mut tester = test();
    let code = "
        let data = $in
        $env.config.table.footer_inheritance = true
        $env.config.footer_mode = 7
        $data
        | table --expand --width=80
    ";

    tester
        .run_with_data(
            code,
            test_value!([
                { a: "kv", b: { "0": 0, "1": 1, "2": 2, "3": 3, "4": 4 } },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────╮
            │ # │  a   │     b     │
            ├───┼──────┼───────────┤
            │ 0 │ kv   │ ╭───┬───╮ │
            │   │      │ │ 0 │ 0 │ │
            │   │      │ │ 1 │ 1 │ │
            │   │      │ │ 2 │ 2 │ │
            │   │      │ │ 3 │ 3 │ │
            │   │      │ │ 4 │ 4 │ │
            │   │      │ ╰───┴───╯ │
            │ 1 │ data │         0 │
            │ 2 │ data │         0 │
            ╰───┴──────┴───────────╯
        "})?;

    tester
        .run_with_data(
            code,
            test_value!([
                { a: "kv", b: { "0": 0, "1": 1, "2": 2, "3": 3, "4": 4, "5": 5 } },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────╮
            │ # │  a   │     b     │
            ├───┼──────┼───────────┤
            │ 0 │ kv   │ ╭───┬───╮ │
            │   │      │ │ 0 │ 0 │ │
            │   │      │ │ 1 │ 1 │ │
            │   │      │ │ 2 │ 2 │ │
            │   │      │ │ 3 │ 3 │ │
            │   │      │ │ 4 │ 4 │ │
            │   │      │ │ 5 │ 5 │ │
            │   │      │ ╰───┴───╯ │
            │ 1 │ data │         0 │
            │ 2 │ data │         0 │
            ├───┼──────┼───────────┤
            │ # │  a   │     b     │
            ╰───┴──────┴───────────╯
        "})
}

#[test]
fn table_footer_inheritance_list_rows() -> Result {
    let mut tester = test();
    let code = "
        let data = $in
        $env.config.table.footer_inheritance = true
        $env.config.footer_mode = 7
        $data
        | table --expand --width=80
    ";

    tester
        .run_with_data(
            code,
            test_value!([
                { a: "kv", b: { "0": (test_table![["field"]; [0], [1], [2], [3], [4]]) } },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────────────────╮
            │ # │  a   │           b           │
            ├───┼──────┼───────────────────────┤
            │ 0 │ kv   │ ╭───┬───────────────╮ │
            │   │      │ │   │ ╭───┬───────╮ │ │
            │   │      │ │ 0 │ │ # │ field │ │ │
            │   │      │ │   │ ├───┼───────┤ │ │
            │   │      │ │   │ │ 0 │     0 │ │ │
            │   │      │ │   │ │ 1 │     1 │ │ │
            │   │      │ │   │ │ 2 │     2 │ │ │
            │   │      │ │   │ │ 3 │     3 │ │ │
            │   │      │ │   │ │ 4 │     4 │ │ │
            │   │      │ │   │ ╰───┴───────╯ │ │
            │   │      │ ╰───┴───────────────╯ │
            │ 1 │ data │                     0 │
            │ 2 │ data │                     0 │
            ╰───┴──────┴───────────────────────╯
        "})?;

    tester
        .run_with_data(
            code,
            test_value!([
                { a: "kv", b: { "0": (test_table![["field"]; [0], [1], [2], [3], [4], [5]]) } },
                { a: "data", b: 0 },
                { a: "data", b: 0 },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────┬───────────────────────╮
            │ # │  a   │           b           │
            ├───┼──────┼───────────────────────┤
            │ 0 │ kv   │ ╭───┬───────────────╮ │
            │   │      │ │   │ ╭───┬───────╮ │ │
            │   │      │ │ 0 │ │ # │ field │ │ │
            │   │      │ │   │ ├───┼───────┤ │ │
            │   │      │ │   │ │ 0 │     0 │ │ │
            │   │      │ │   │ │ 1 │     1 │ │ │
            │   │      │ │   │ │ 2 │     2 │ │ │
            │   │      │ │   │ │ 3 │     3 │ │ │
            │   │      │ │   │ │ 4 │     4 │ │ │
            │   │      │ │   │ │ 5 │     5 │ │ │
            │   │      │ │   │ ╰───┴───────╯ │ │
            │   │      │ ╰───┴───────────────╯ │
            │ 1 │ data │                     0 │
            │ 2 │ data │                     0 │
            ├───┼──────┼───────────────────────┤
            │ # │  a   │           b           │
            ╰───┴──────┴───────────────────────╯
        "})
}
/// Test checking whether automatic table rendering correctly uses ansi coloring.
#[test]
fn table_colors() -> Result {
    let mut tester = test();
    let colored = indoc! {"
        \u{1b}[39m╭───┬───╮\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32ma\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[39m1\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32mb\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[39m2\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m╰───┴───╯\u{1b}[0m"};

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.use_ansi_coloring = true
                $data | table
            ",
            test_value!({a: 1, b: 2}),
        )
        .expect_value_eq(colored)?;

    tester
        .run_with_data(
            "
                let data = $in
                $env.config.use_ansi_coloring = false
                $data | table
            ",
            test_value!({a: 1, b: 2}),
        )
        .expect_value_eq(indoc! {"
            ╭───┬───╮
            │ a │ 1 │
            │ b │ 2 │
            ╰───┴───╯"})
}

#[test]
fn table_empty_colors() -> Result {
    let mut tester = test();
    let empty_list_colored = indoc! {"
        \u{1b}[39m╭────────────╮\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[2mempty list\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m╰────────────╯\u{1b}[0m
    "};
    let empty_record_colored = indoc! {"
        \u{1b}[39m╭──────────────╮\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[2mempty record\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m╰──────────────╯\u{1b}[0m"};

    tester
        .run("$env.config.use_ansi_coloring = true; [] | table")
        .expect_value_eq(empty_list_colored)?;

    tester
        .run("$env.config.use_ansi_coloring = true; {} | table")
        .expect_value_eq(empty_record_colored)?;

    tester
        .run("$env.config.use_ansi_coloring = false; [] | table")
        .expect_value_eq(indoc! {"
            ╭────────────╮
            │ empty list │
            ╰────────────╯
        "})?;

    tester
        .run("$env.config.use_ansi_coloring = false; {} | table")
        .expect_value_eq(indoc! {"
            ╭──────────────╮
            │ empty record │
            ╰──────────────╯"})
}

#[test]
fn table_index() -> Result {
    test()
        .run_with_data(
            "table --width=80",
            test_table![
                ["index", "var"];
                ["abc", 1],
                ["def", 2],
                ["ghi", 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭─────┬─────╮
            │   # │ var │
            ├─────┼─────┤
            │ abc │   1 │
            │ def │   2 │
            │ ghi │   3 │
            ╰─────┴─────╯
        "})
}

#[test]
fn table_index_expand() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand",
            test_table![
                ["index", "var"];
                ["abc", 1],
                ["def", 2],
                ["ghi", 3],
            ],
        )
        .expect_value_eq(indoc! {"
            ╭─────┬─────╮
            │   # │ var │
            ├─────┼─────┤
            │ abc │   1 │
            │ def │   2 │
            │ ghi │   3 │
            ╰─────┴─────╯
        "})
}

#[test]
fn table_expand_big_header() -> Result {
    let actual: String = test().run(
        "
        let column_name = (('' | fill -c 'a' --width 81))
        [{ $column_name: 'contents' }]
        | table -e --width=80
    ",
    )?;

    assert_eq!(
        actual,
        indoc! {"
            ╭───┬──────────────────────────────────────────────────────────────────────────╮
            │ # │ aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa │
            │   │ aaaaaaaaa                                                                │
            ├───┼──────────────────────────────────────────────────────────────────────────┤
            │ 0 │ contents                                                                 │
            ╰───┴──────────────────────────────────────────────────────────────────────────╯
        "}
    );
    Ok(())
}

#[rstest]
fn table_missing_value(#[values(false, true)] expand: bool) -> Result {
    let mut tester = test();
    let data: Value = tester.run("[{foo: '____________________'} {} {}]")?;
    let () = tester.run_with_data("let expand = $in", expand)?;
    let rendered: String = tester.run_with_data("table --expand=$expand | ansi strip", data)?;
    pretty_assertions::assert_str_eq!(
        rendered,
        "╭───┬──────────────────────╮\n\
         │ # │         foo          │\n\
         ├───┼──────────────────────┤\n\
         │ 0 │ ____________________ │\n\
         │ 1 │          ❎          │\n\
         │ 2 │          ❎          │\n\
         ╰───┴──────────────────────╯\n",
    );
    Ok(())
}

#[rstest]
#[case::off(false, 3)]
#[case::on(true, 1)]
fn horizontal_alignment_with_header_on_separator(
    #[case] header_on_separator: bool,
    #[case] skip: usize,
    #[values(false, true)] expand: bool,
) -> Result {
    let mut tester = test();

    let () = tester.run("$env.config.footer_mode = 'never'")?;
    let () = tester.run_with_data(
        "$env.config.table.header_on_separator = $in",
        header_on_separator,
    )?;
    let () = tester.run_with_data("let expand = $in", expand)?;

    let data: Value = {
        let code = r#"[
            { align:      "_", val: "__________" }
            { align:   "left", val:         "a"  }
            { align:  "right", val:           0  }
            { align:   "left", val:         "a"  }
            { align: "center",                   }
            { align:   "left", val:         "a"  }
            { align: "center",                   }
            { align:  "right", val:           0  }
        ]"#;
        tester.run(code)?
    };

    let rendered: String = tester.run_with_data("table --expand=$expand | ansi strip", data)?;
    let trimmed = {
        let mut positions = rendered.as_bytes().iter().positions(|b| *b == b'\n');
        let start = positions.nth(skip - 1).unwrap() + 1;
        let end = positions.nth_back(1).unwrap() + 1;
        &rendered[start..end]
    };

    let expected = indoc! {"
        │ 0 │ _      │ __________ │
        │ 1 │ left   │ a          │
        │ 2 │ right  │          0 │
        │ 3 │ left   │ a          │
        │ 4 │ center │     ❎     │
        │ 5 │ left   │ a          │
        │ 6 │ center │     ❎     │
        │ 7 │ right  │          0 │
    "};

    pretty_assertions::assert_str_eq!(trimmed, expected);
    Ok(())
}

#[test]
fn table_missing_value_custom() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.missing_value_symbol = 'NULL'
                $data | table
            ",
            test_value!([{foo: ()}, {}, {}]),
        )
        .expect_value_eq(indoc! {"
            ╭───┬──────╮
            │ # │ foo  │
            ├───┼──────┤
            │ 0 │      │
            │ 1 │ NULL │
            │ 2 │ NULL │
            ╰───┴──────╯
        "})
}

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
fn table_index_column_with_index_flag_false() -> Result {
    test()
        .run_with_data(
            "table --index false --width 80",
            test_value!([{index: 0, data: "yes"}]),
        )
        .expect_value_eq(indoc! {"
            ╭───────┬──────╮
            │ index │ data │
            ├───────┼──────┤
            │     0 │ yes  │
            ╰───────┴──────╯
        "})
}

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
            test_value!([{name: "src"}]),
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
            test_value!([{dir: "src", file: "main.rs"}]),
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
            test_value!([{dir: "src", file: "main.rs"}]),
        )
        .expect_value_eq(expected)
}
