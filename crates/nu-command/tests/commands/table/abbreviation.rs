use indoc::indoc;
use nu_test_support::prelude::*;
use rstest::rstest;

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
    let top = "╭──────┬──────╮";
    let bottom = "╰──────┴──────╯";
    let output = format!("{top}\n{rows}\n{bottom}\n");
    let actual: String = test().run("0..2000 | table --width=80 -a 2000")?;
    assert_eq!(actual, output);
    let actual: String = test().run("0..2000 | table --width=80 -a 200000")?;
    assert_eq!(actual, output);
    Ok(())
}
