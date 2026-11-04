use indoc::indoc;
use nu_test_support::prelude::*;

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
            [[a]; [11111111111111111111111111111111111111111111111111111111111111111111111111111111.0]]
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
fn table_collapse_aligns_rows_with_nested_lists() -> Result {
    test()
        .run("[[a b]; [1 [2 2]] [3 [4 4]]] | table --width=80 --collapse")
        .expect_value_eq(indoc! {"
            ╭───┬───╮
            │ a │ b │
            ├───┼───┤
            │ 1 │ 2 │
            │   ├───┤
            │   │ 2 │
            ├───┼───┤
            │ 3 │ 4 │
            │   ├───┤
            │   │ 4 │
            ╰───┴───╯
        "})
}

#[test]
fn table_collapse_aligns_nested_tables() -> Result {
    test()
        .run(r#"[[a b c]; [[1 12345 1] [["444444" "hello\nworld"]; [3 4]] 1] [5 [6 7] 2]] | table --width=80 --collapse"#)
        .expect_value_eq(indoc! {"
            ╭───────┬────────────────┬───╮
            │ a     │ b              │ c │
            ├───────┼────────┬───────┼───┤
            │ 1     │ 444444 │ hello │ 1 │
            ├───────┤        │ world │   │
            │ 12345 ├────────┼───────┤   │
            ├───────┤ 3      │ 4     │   │
            │ 1     │        │       │   │
            ├───────┼────────┴───────┼───┤
            │ 5     │ 6              │ 2 │
            │       ├────────────────┤   │
            │       │ 7              │   │
            ╰───────┴────────────────┴───╯
        "})
}

#[test]
fn table_collapse_aligns_multiline_values() -> Result {
    test()
        .run(r#"[[a b]; ["x\ny\nz" 1] [2 "p\nq"]] | table --width=80 --collapse"#)
        .expect_value_eq(indoc! {"
            ╭───┬───╮
            │ a │ b │
            ├───┼───┤
            │ x │ 1 │
            │ y │   │
            │ z │   │
            ├───┼───┤
            │ 2 │ p │
            │   │ q │
            ╰───┴───╯
        "})
}

#[test]
fn table_collapse_aligns_wrapped_values() -> Result {
    test()
        .run(r#"[[a b c]; [1 x "some longer text"] [2 y 3]] | table --width=24 --collapse"#)
        .expect_value_eq(indoc! {"
            ╭───┬───┬──────────────╮
            │ a │ b │ c            │
            ├───┼───┼──────────────┤
            │ 1 │ x │ some longer  │
            │   │   │ text         │
            ├───┼───┼──────────────┤
            │ 2 │ y │ 3            │
            ╰───┴───┴──────────────╯
        "})
}

#[test]
fn table_collapse_aligns_truncated_tables() -> Result {
    test()
        .run("[[a b c d]; [1 [2 2] 3 4] [5 [6 6 6] 7 8]] | table --width=15 --collapse")
        .expect_value_eq(indoc! {"
            ╭───┬───┬─────╮
            │ a │ b │ ... │
            ├───┼───┤     │
            │ 1 │ 2 │     │
            │   ├───┤     │
            │   │ 2 │     │
            ├───┼───┤     │
            │ 5 │ 6 │     │
            │   ├───┤     │
            │   │ 6 │     │
            │   ├───┤     │
            │   │ 6 │     │
            ╰───┴───┴─────╯
        "})
}

#[test]
fn table_collapse_aligns_multiline_record_keys() -> Result {
    test()
        .run(r#"{"aa\nb": 1, ccc: 2} | table --width=80 --collapse"#)
        .expect_value_eq(indoc! {"
            ╭─────┬───╮
            │ aa  │ 1 │
            │ b   │   │
            ├─────┼───┤
            │ ccc │ 2 │
            ╰─────┴───╯"})
}

#[test]
fn table_collapse_none_aligns_nested_tables() -> Result {
    test()
        .run(r#"$env.config.table.mode = "none"; [[a b]; [x [[c d]; [1 "ab\ncd"]]]] | table --width=80 --collapse"#)
        .expect_value_eq(indoc! {"
            \x20a  b     
            \x20x  c  d  
            \x20   1  ab 
            \x20      cd 
        "})
}

#[test]
fn table_collapse_padding_aligns_nested_tables() -> Result {
    test()
        .run(r#"$env.config.table.padding = {left: 2, right: 2}; [[a b]; [[1 12345] [["2" "hello\nworld"]; [3 4]]] [5 [6 7]]] | table --width=80 --collapse"#)
        .expect_value_eq(indoc! {"
            ╭─────────┬───────────────╮
            │  a      │  b            │
            ├─────────┼─────┬─────────┤
            │  1      │  2  │  hello  │
            ├─────────┤     │  world  │
            │  12345  ├─────┼─────────┤
            │         │  3  │  4      │
            ├─────────┼─────┴─────────┤
            │  5      │  6            │
            │         ├───────────────┤
            │         │  7            │
            ╰─────────┴───────────────╯
        "})
}

#[test]
fn table_collapse_truncation_fits_width_around_empty_list() -> Result {
    test()
        .run("[[a b c]; [x {kkkkkkkk: []} y]] | table --width=13 --collapse")
        .expect_value_eq(indoc! {"
            ╭───┬─────╮
            │ a │ ... │
            ├───┤     │
            │ x │     │
            ╰───┴─────╯
        "})
}

#[test]
fn table_collapse_truncation_counts_padding() -> Result {
    test()
        .run("$env.config.table.padding = {left: 2, right: 2}; [[a b c]; [1 2 3]] | table --width=16 --collapse")
        .expect_value_eq(indoc! {"
            ╭─────┬───────╮
            │  a  │  ...  │
            ├─────┤       │
            │  1  │       │
            ╰─────┴───────╯
        "})
}

#[test]
fn table_collapse_padding_zero_keeps_empty_cells() -> Result {
    test()
        .run(r#"$env.config.table.padding = {left: 0, right: 0}; [[u v]; [{e: ""} x]] | table --width=80 --collapse"#)
        .expect_value_eq(indoc! {"
            ╭───┬─╮
            │u  │v│
            ├─┬─┼─┤
            │e│ │x│
            ╰─┴─┴─╯
        "})
}

#[test]
fn table_collapse_restructured() -> Result {
    test()
        .run(r#"$env.config.table.mode = "restructured"; [[a b]; [1 2] [3 4]] | table --width=80 --collapse"#)
        .expect_value_eq(indoc! {"
            ======
            \x20a  b 
            ======
            \x201  2 
            ======
            \x203  4 
            ======
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
            ╭───┬────────────────────────────────────────────────────┬──────┬──────────────╮
            │ # │                         a                          │  b   │      c       │
            ├───┼────────────────────────────────────────────────────┼──────┼──────────────┤
            │ 0 │ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx     │    2 │            3 │
            │ 1 │                                                  4 │    5 │ ╭───┬───╮    │
            │   │                                                    │      │ │ 0 │ 1 │    │
            │   │                                                    │      │ │ 1 │ 2 │    │
            │   │                                                    │      │ │ 2 │ 3 │    │
            │   │                                                    │      │ ╰───┴───╯    │
            ╰───┴────────────────────────────────────────────────────┴──────┴──────────────╯
        "})
}

#[test]
fn table_expand_deep_0() -> Result {
    test()
        .run_with_data(
            "table --width=80 --expand --expand-deep=1",
            test_value!([
                { a: 1, b: 2, c: 3 },
                { a: 4, b: 5, c: [1, 2, [1, 2, 3]] },
            ]),
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
            test_value!([
                { a: 1, b: 2, c: 3 },
                { a: 4, b: 5, c: [1, 2, [1, 2, 3]] },
            ]),
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
            test_value!([
                { a: 1, b: 2, c: 3 },
                { a: 4, b: 5, c: [1, 2, [1, 1, 1]] },
            ]),
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
            test_value!([
                { a: 1, b: 2, c: 3 },
                { a: 4, b: 5, c: [1, 2, [1, 1, 1]] },
            ]),
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
                { a: 4, b: 5, c: [1, 2, [1, [1, 1, 1], 1]] },
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
        .run_with_data(
            "table --width=80 --expand",
            [test_value!({
                c: { d: 1 },
            })],
        )
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
                { a: 4, b: 5, c: [1, 2, { a: 123, b: 234, c: 345 }] },
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
        [
            test_value!({
                f1: "a string",
                f2: 1000,
            }),
            1,
            2,
        ],
    ];
    test()
        .run_with_data(
            "table --width=80 --expand",
            test_value!({
                field1: ["a", "b", "c"],
                field2: [123, 234, 345],
                field3: (field3),
                field4: {
                    f1: 1,
                    f2: 3,
                    f3: {
                        f1: "f1",
                        f2: "f2",
                        f3: "f3",
                    },
                },
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
fn table_expand_nested_error_list_does_not_rethrow() -> Result {
    // Nested `Value::Error` lists (e.g. `$ans.last` after `str length` on a table)
    // must render under `--expand` instead of rethrowing. Construct the record in
    // Rust: collecting `[{a: 1}] | str length` in a subexpression raises first.
    use nu_protocol::shell_error::generic::GenericError;
    use nu_protocol::{Span, record};

    let err = Value::error(
        ShellError::Generic(GenericError::new("boom", "test", Span::test_data())),
        Span::test_data(),
    );
    let data = Value::test_record(record! {
        "last" => Value::test_list(vec![err.clone(), err]),
    });
    let _: String = test().run_with_data("table --expand | ansi strip", data)?;
    Ok(())
}

#[test]
fn table_expand_top_level_error_list_still_throws() -> Result {
    test()
        .run("[{a: 1}] | str length | table --expand")
        .expect_error_code_eq("nu::shell::only_supports_this_input_type")
}

#[test]
#[deps(TESTBIN_MEOW)]
fn external_with_too_much_stdout_should_not_hang_nu() -> Result {
    use nu_test_support::fs::Stub::FileWithContent;

    Playground::setup("external with too much stdout", |dirs, sandbox| {
        let bytes: usize = 81920;
        let mut large_file_body = String::with_capacity(bytes);
        for _ in 0..bytes {
            large_file_body.push('a');
        }
        sandbox.with_files(&[FileWithContent("a_large_file.txt", &large_file_body)]);
        let actual: String = test().cwd(dirs.test()).run(
            "
            meow a_large_file.txt | table --width=80
        ",
        )?;
        assert_eq!(actual, large_file_body);
        let actual: String = test()
            .cwd(dirs.test())
            .run("let x = meow a_large_file.txt; $x")?;
        assert_eq!(actual, large_file_body);
        Ok(())
    })
}
