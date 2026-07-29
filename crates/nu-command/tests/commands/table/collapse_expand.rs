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
