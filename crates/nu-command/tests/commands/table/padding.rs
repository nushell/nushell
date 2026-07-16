use indoc::indoc;
use nu_test_support::prelude::*;

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
