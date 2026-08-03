use indoc::indoc;
use itertools::Itertools;
use nu_test_support::prelude::*;
use rstest::rstest;

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
            test_value!({
                a: 1,
                b: 2,
            }),
        )
        .expect_value_eq(colored)?;
    tester
        .run_with_data(
            "
                let data = $in
                $env.config.use_ansi_coloring = false
                $data | table
            ",
            test_value!({
                a: 1,
                b: 2,
            }),
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
            test_value!([
                { foo: () },
                {},
                {},
            ]),
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
