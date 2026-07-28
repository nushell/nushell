use indoc::indoc;
use nu_test_support::prelude::*;
use rstest::rstest;

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
fn table_index_column_with_index_flag_false() -> Result {
    test()
        .run_with_data(
            "table --index false --width 80",
            test_value!([
                { index: 0, data: "yes" },
            ]),
        )
        .expect_value_eq(indoc! {"
            ╭───────┬──────╮
            │ index │ data │
            ├───────┼──────┤
            │     0 │ yes  │
            ╰───────┴──────╯
        "})
}
