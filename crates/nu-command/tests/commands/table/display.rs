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

fn issue_18966_table() -> Value {
    test_value!([{
        name: "whatever",
        type: "symlink",
        target: "/some/path",
        readonly: false,
        mode: "rwxrwxrwx",
        num_links: 1,
        inode: 12653657,
        user: "gibbert",
        group: "gibbert",
        size: 10,
        created: "2026-04-03T22:09:22.526691315+07:00",
        accessed: "2026-09-04T18:28:33.313157179+07:00",
        modified: "2026-04-03T22:09:22.526691315+07:00",
    }])
}

#[test]
fn table_trim_wrapping_wraps_squeezed_column() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.trim = { methodology: 'wrapping', wrapping_try_keep_words: true }
                $data | table --width 110 --theme basic
            ",
            issue_18966_table(),
        )
        .expect_value_eq(indoc! {"
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+--------+-----+
            | # |   name   |  type   |   target   | readonly |   mode    | num_links |  inode   |  user   | group  | ... |
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+--------+-----+
            | 0 | whatever | symlink | /some/path | false    | rwxrwxrwx |         1 | 12653657 | gibbert | gibber | ... |
            |   |          |         |            |          |           |           |          |         | t      |     |
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+--------+-----+
        "})
}

#[test]
fn table_trim_truncating_keeps_squeezed_column_on_one_line() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.trim = { methodology: 'truncating', truncating_suffix: '...' }
                $data | table --width 110 --theme basic
            ",
            issue_18966_table(),
        )
        .expect_value_eq(indoc! {"
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+--------+-----+
            | # |   name   |  type   |   target   | readonly |   mode    | num_links |  inode   |  user   | group  | ... |
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+--------+-----+
            | 0 | whatever | symlink | /some/path | false    | rwxrwxrwx |         1 | 12653657 | gibbert | gib... | ... |
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+--------+-----+
        "})
}

#[test]
fn table_trim_overflow_without_squeezed_column_matches_for_wrap_and_truncate() -> Result {
    let expected = indoc! {"
        +---+----------+---------+------------+----------+-----------+-----+
        | # |   name   |  type   |   target   | readonly |   mode    | ... |
        +---+----------+---------+------------+----------+-----------+-----+
        | 0 | whatever | symlink | /some/path | false    | rwxrwxrwx | ... |
        +---+----------+---------+------------+----------+-----------+-----+
    "};

    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.trim = { methodology: 'wrapping', wrapping_try_keep_words: true }
                $data | table --width 70 --theme basic
            ",
            issue_18966_table(),
        )
        .expect_value_eq(expected)?;

    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.trim = { methodology: 'truncating', truncating_suffix: '...' }
                $data | table --width 70 --theme basic
            ",
            issue_18966_table(),
        )
        .expect_value_eq(expected)
}

#[test]
fn table_trim_truncating_stays_one_line_above_width_threshold() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.trim = { methodology: 'truncating', truncating_suffix: '...' }
                $data | table --width 160 --theme basic
            ",
            issue_18966_table(),
        )
        .expect_value_eq(indoc! {"
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+---------+------+-------------------------------------+-----+
            | # |   name   |  type   |   target   | readonly |   mode    | num_links |  inode   |  user   |  group  | size |               created               | ... |
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+---------+------+-------------------------------------+-----+
            | 0 | whatever | symlink | /some/path | false    | rwxrwxrwx |         1 | 12653657 | gibbert | gibbert |   10 | 2026-04-03T22:09:22.526691315+07:00 | ... |
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+---------+------+-------------------------------------+-----+
        "})
}

/// Interior width of each column from a basic-theme top border (`+---+----+`).
fn basic_column_inner_widths(table: &str) -> Vec<usize> {
    let Some(border) = table.lines().find(|line| line.starts_with('+')) else {
        return Vec::new();
    };
    border
        .split('+')
        .filter(|seg| !seg.is_empty())
        .map(|seg| seg.chars().count())
        .collect()
}

/// Inner widths from a body row (`│ ... │ ... │`). Works for rounded theme
/// and `header_on_separator`, where the top border is mixed with header text.
fn data_row_inner_widths(table: &str) -> Vec<usize> {
    let Some(row) = table.lines().find(|line| {
        line.contains('│')
            && (line.contains(" file ") || line.contains(" dir ") || line.contains(" symlink "))
    }) else {
        return Vec::new();
    };
    row.split('│')
        .skip(1)
        .filter(|seg| !seg.is_empty())
        .map(|seg| seg.chars().count())
        .collect()
}

fn overflow_ls_like_table() -> Value {
    test_value!([{
        name: "grok-continue-115-caching-regression.txt",
        type: "file",
        target: "",
        readonly: false,
        mode: "rw-r--r--",
        num_links: 1,
        inode: 12345678,
        user: "fdncred",
        group: "staff",
        size: 128,
        created: "2026-01-01T00:00:00",
        accessed: "2026-01-01T00:00:00",
        modified: "2026-01-01T00:00:00",
    }])
}

const LS_LIKE_ROWS: &str = r#"
    [
        {
            name: "tango"
            type: "dir"
            target: ""
            readonly: false
            mode: "rwxr-xr-x"
            num_links: 4
            inode: 216175737
            user: "fdncred"
            group: "staff"
            size: 128b
            created: "2026-04-03T22:09:22"
            accessed: "2026-09-04T18:28:33"
            modified: "2026-04-03T22:09:22"
        }
        {
            name: "crates/nu-command/tests/commands/table/display.rs"
            type: "file"
            target: ""
            readonly: false
            mode: "rw-r--r--"
            num_links: 1
            inode: 235854364
            user: "fdncred"
            group: "staff"
            size: 256b
            created: "2026-04-03T22:09:22"
            accessed: "2026-09-04T18:28:33"
            modified: "2026-04-03T22:09:22"
        }
    ]
"#;

#[rstest]
fn table_trim_scenario_grid(
    #[values("wrapping", "truncating")] methodology: &str,
    #[values(false, true)] header_on_separator: bool,
    #[values(40, 50, 60, 70, 80, 90, 100, 110, 120, 140, 160)] width: usize,
) -> Result {
    const PAD: usize = 2;
    const MIN_CONTENT: usize = 4;
    let code = format!(
        "
            $env.config.footer_mode = 'always'
            $env.config.table.header_on_separator = {header_on_separator}
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '...'
            }}
            {LS_LIKE_ROWS} | table --width {width} --theme basic
        "
    );
    let rendered: String = test().run(code)?;
    let maxline = rendered
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let nlines = rendered.lines().count();
    let inners = basic_column_inner_widths(&rendered);

    // Skip index (first) and trailing `...` (last, inner width 5 with pad 2 + 3 dots).
    let data_inners: Vec<usize> = if inners.len() >= 2 {
        let last = *inners.last().unwrap_or(&0);
        let end = if last <= 5 {
            inners.len() - 1
        } else {
            inners.len()
        };
        inners[1..end].to_vec()
    } else {
        Vec::new()
    };
    let min_inner = data_inners.iter().copied().min().unwrap_or(0);
    let min_content = min_inner.saturating_sub(PAD);

    assert!(
        maxline <= width,
        "line {maxline} > width {width}\n{rendered}"
    );
    if !data_inners.is_empty() {
        assert!(
            min_content >= MIN_CONTENT,
            "data col content {min_content} < {MIN_CONTENT}\n{rendered}"
        );
    }
    if methodology == "truncating" {
        // 2 borders + header + sep + 2 records + footer + sep = 8 lines
        // when header_on_separator, header/footer sit on borders: 2+2 records+seps
        let max_ok = if header_on_separator { 8 } else { 10 };
        assert!(
            nlines <= max_ok,
            "truncating wrapped: {nlines} lines\n{rendered}"
        );
    }
    Ok(())
}

/// Default display at >= 100 columns is `table -e`.
#[rstest]
fn expand_ls_like_table_stays_readable(
    #[values("wrapping", "truncating")] methodology: &str,
    #[values(false, true)] header_on_separator: bool,
    #[values(60, 70, 80, 90, 100, 110, 120, 140, 146)] width: usize,
) -> Result {
    let code = format!(
        "
            let data = $in
            $env.config.footer_mode = 'always'
            $env.config.table.header_on_separator = {header_on_separator}
            $env.config.table.trim = {{
                methodology: '{methodology}'
                wrapping_try_keep_words: true
                truncating_suffix: '>>'
            }}
            $data | table --expand --width {width}
        "
    );
    let rendered: String = test().run_with_data(code, overflow_ls_like_table())?;
    let maxline = rendered
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let nlines = rendered.lines().count();
    let inners = data_row_inner_widths(&rendered);
    let data_inners: Vec<usize> = if inners.len() >= 2 {
        let last = *inners.last().unwrap_or(&0);
        let end = if last <= 5 {
            inners.len() - 1
        } else {
            inners.len()
        };
        inners[1..end].to_vec()
    } else {
        Vec::new()
    };

    assert!(
        maxline <= width,
        "line {maxline} > width {width}\n{rendered}"
    );
    assert!(
        !data_inners.contains(&1),
        "1-character data column\n{rendered}"
    );
    assert_contains_not("│ s │", &rendered);
    assert_contains_not("│ i │", &rendered);
    assert_contains_not("│ z │", &rendered);
    assert_contains_not("│ e │", &rendered);
    if methodology == "truncating" && width >= 140 {
        let data_rows = rendered
            .lines()
            .filter(|line| line.contains('│') && (line.contains("dir") || line.contains("file")))
            .count();
        assert!(
            nlines <= data_rows * 3 + 10,
            "truncating looks wrapped: {nlines} lines\n{rendered}"
        );
    }
    Ok(())
}

/// Default display at >= 100 columns is `table -e`. Leftover after a long
/// `name` used to become a 1-character wrapping `size` column.
#[test]
fn expand_truncating_leftover_is_not_a_one_char_size_column() -> Result {
    let code = r#"
        $env.config.footer_mode = 'always'
        $env.config.table.trim = {
            methodology: 'truncating'
            truncating_suffix: '...'
        }
        [
            {
                name: "grok-continue-115-caching-regression.txt"
                type: "file"
                target: ""
                readonly: false
                mode: "rw-r--r--"
                num_links: 1
                inode: 12345678
                user: "fdncred"
                group: "staff"
                size: 128b
                created: 2026-01-01
                accessed: 2026-01-01
                modified: 2026-01-01
            }
        ] | table --expand --width 146
    "#;
    let rendered: String = test().run(code)?;
    let inners = data_row_inner_widths(&rendered);
    let data_inners: Vec<usize> = if inners.len() >= 2 {
        let last = *inners.last().unwrap_or(&0);
        let end = if last <= 5 {
            inners.len() - 1
        } else {
            inners.len()
        };
        inners[1..end].to_vec()
    } else {
        Vec::new()
    };
    assert!(
        !data_inners.contains(&1),
        "1-character data column\n{rendered}"
    );
    assert_contains_not("│ s │", &rendered);
    assert_contains_not("│ i │", &rendered);
    assert_contains_not("│ z │", &rendered);
    assert_contains_not("│ e │", &rendered);
    Ok(())
}

/// Wrapping vs truncating must differ for every layout that `table` uses
/// when columns overflow: expand on/off, header on separator, footer,
/// keep-words, and typical terminal widths. Default display at >= 100
/// columns is `table -e`.
#[rstest]
fn wrap_and_truncate_differ_for_each_table_layout(
    #[values(false, true)] expand: bool,
    #[values(false, true)] header_on_separator: bool,
    #[values("never", "always")] footer_mode: &str,
    #[values(true, false)] keep_words: bool,
    #[values(80, 110, 146)] width: usize,
) -> Result {
    let expand_flag = if expand { "--expand" } else { "" };
    let wrapping: String = test().run_with_data(
        format!(
            "
                let data = $in
                $env.config.footer_mode = '{footer_mode}'
                $env.config.table.header_on_separator = {header_on_separator}
                $env.config.table.trim = {{
                    methodology: 'wrapping'
                    wrapping_try_keep_words: {keep_words}
                    truncating_suffix: '>>'
                }}
                $data | table {expand_flag} --width {width}
            "
        ),
        overflow_ls_like_table(),
    )?;
    let truncating: String = test().run_with_data(
        format!(
            "
                let data = $in
                $env.config.footer_mode = '{footer_mode}'
                $env.config.table.header_on_separator = {header_on_separator}
                $env.config.table.trim = {{
                    methodology: 'truncating'
                    wrapping_try_keep_words: {keep_words}
                    truncating_suffix: '>>'
                }}
                $data | table {expand_flag} --width {width}
            "
        ),
        overflow_ls_like_table(),
    )?;

    for rendered in [&wrapping, &truncating] {
        assert_contains_not("│ s │", rendered);
        assert_contains_not("│ i │", rendered);
        assert_contains_not("│ z │", rendered);
        assert_contains_not("│ e │", rendered);
        assert!(
            !data_row_inner_widths(rendered).contains(&1),
            "1-character data column\n{rendered}"
        );
    }
    // Below 120 columns without header-on-separator both methodologies use
    // last-column squeeze, so they can match when no cell is actually cut.
    // They must differ when wrapping uses the many-column path: header on
    // separator, or a wide terminal.
    if header_on_separator || width > 120 {
        pretty_assertions::assert_ne!(
            wrapping,
            truncating,
            "wrapping and truncating matched for expand={expand} hos={header_on_separator} footer={footer_mode} keep_words={keep_words} width={width}\n{wrapping}"
        );
    }
    Ok(())
}

#[rstest]
fn wrap_and_truncate_differ_for_kv_tables(
    #[values(false, true)] expand: bool,
    #[values(false, true)] header_on_separator: bool,
    #[values(40, 60)] width: usize,
) -> Result {
    let data = test_record! {
        "key1" => "111111111111111111111111111111111111111111111111111111111111",
    };
    let expand_flag = if expand { "--expand" } else { "" };
    let wrapping: String = test().run_with_data(
        format!(
            "
                let data = $in
                $env.config.table.header_on_separator = {header_on_separator}
                $env.config.table.trim = {{
                    methodology: 'wrapping'
                    wrapping_try_keep_words: true
                    truncating_suffix: '>>'
                }}
                $data | table {expand_flag} --width {width}
            "
        ),
        data.clone(),
    )?;
    let truncating: String = test().run_with_data(
        format!(
            "
                let data = $in
                $env.config.table.header_on_separator = {header_on_separator}
                $env.config.table.trim = {{
                    methodology: 'truncating'
                    wrapping_try_keep_words: true
                    truncating_suffix: '>>'
                }}
                $data | table {expand_flag} --width {width}
            "
        ),
        data,
    )?;
    pretty_assertions::assert_ne!(
        wrapping,
        truncating,
        "KV wrapping and truncating matched for expand={expand} hos={header_on_separator} width={width}\n{wrapping}"
    );
    assert!(
        wrapping.lines().count() > truncating.lines().count(),
        "KV wrapping should wrap onto extra lines\nwrapping:\n{wrapping}\ntruncating:\n{truncating}"
    );
    Ok(())
}

#[test]
fn table_trim_header_on_separator_keeps_header_text() -> Result {
    test()
        .run_with_data(
            "
                let data = $in
                $env.config.table.header_on_separator = true
                $env.config.table.trim = { methodology: 'truncating', truncating_suffix: '...' }
                $data | table --width 110 --theme basic
            ",
            issue_18966_table(),
        )
        .expect_value_eq(indoc! {"
            +-#-+---name---+--type---+---target---+-readonly-+---mode----+-num_links-+--inode---+--user---+-group--+-...-+
            | 0 | whatever | symlink | /some/path | false    | rwxrwxrwx |         1 | 12653657 | gibbert | gib... | ... |
            +---+----------+---------+------------+----------+-----------+-----------+----------+---------+--------+-----+
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

/// Primitive custom values (e.g. semver) keep their type-specific color when listed.
/// Structured custom values are still expanded for table layout.
#[test]
fn table_semver_list_colors() -> Result {
    let mut tester = test();
    // cyan_bold = bold cyan (1;36)
    let colored = indoc! {"
        \u{1b}[39m╭───┬───────╮\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m0\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[1;36m1.0.0\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m│\u{1b}[0m \u{1b}[1;32m1\u{1b}[0m \u{1b}[39m│\u{1b}[0m \u{1b}[1;36m2.0.0\u{1b}[0m \u{1b}[39m│\u{1b}[0m
        \u{1b}[39m╰───┴───────╯\u{1b}[0m
    "};
    tester
        .run(
            "
                $env.config.use_ansi_coloring = true
                ['1.0.0' '2.0.0'] | into semver | table
            ",
        )
        .expect_value_eq(colored)
}

/// A lone primitive custom (semver) prints as a scalar, not a one-row table.
#[test]
fn table_semver_single_is_scalar() -> Result {
    let mut tester = test();
    tester
        .run("'1.0.0' | into semver | table")
        .expect_value_eq("1.0.0")?;
    tester
        .run("'1.0.0' | into semver | describe")
        .expect_value_eq("semver")
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

#[test]
fn expand_truncate_keeps_each_line_of_a_multiline_cell() -> Result {
    let rendered: String = test().run(
        r#"
            $env.config.table.trim = {
                methodology: 'truncating'
                truncating_suffix: '>>'
            }
            [{ note: "hello\nworld-is-a-very-long-line" }] | table --expand --width 24
        "#,
    )?;
    assert_contains("hello", &rendered);
    assert_contains("world", &rendered);
    assert_contains(">>", &rendered);
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
