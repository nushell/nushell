use indoc::indoc;
use nu_test_support::prelude::*;
use rstest::rstest;

const WIDTH_PRIORITY_EIGHT_COL_INPUT: &str = indoc! {"
    [
        [a b c d e f g h];
        [
            value_0000000000000000 value_1111111111111111 value_2222222222222222
            value_3333333333333333 value_4444444444444444 value_5555555555555555
            priority_value_12345 value_7777777777777777
        ]
        [
            value_0000000000000000 value_1111111111111111 value_2222222222222222
            value_3333333333333333 value_4444444444444444 value_5555555555555555
            priority_value_12345 value_7777777777777777
        ]
    ]
"};

const WIDTH_PRIORITY_RECORD_INPUT: &str = indoc! {"
    [
        {
            c0: v00000000000 c1: v11111111111 c2: v22222222222 c3: v33333333333
            c4: v44444444444 c5: v55555555555 c6: v66666666666 c7: v77777777777
            c8: v88888888888 c9: v99999999999
        }
        {
            c0: v00000000000 c1: v11111111111 c2: v22222222222 c3: v33333333333
            c4: v44444444444 c5: v55555555555 c6: v66666666666 c7: v77777777777
            c8: v88888888888 c9: v99999999999
        }
    ]
"};

const WIDTH_PRIORITY_NAME_INPUT: &str = indoc! {"
    [
        [name type target readonly mode num_links inode user group size created accessed modified];
        [
            very_very_very_long_filename_that_should_get_priority_and_avoid_wrapping.txt
            file '' false rw-r--r-- 1 12345 me staff 1234
            '2 years ago' '2 years ago' '2 years ago'
        ]
        [
            another_extremely_long_name_for_priority_column_display.txt
            file '' false rw-r--r-- 1 54321 me staff 5678
            '2 years ago' '2 years ago' '2 years ago'
        ]
    ]
"};

const TABLE_CFG_HEADER_SEPARATOR: &str = "{ table: { header_on_separator: true } }";
const TABLE_CFG_BASIC_NO_INDEX: &str =
    "{ table: { mode: basic, index_mode: never, header_on_separator: false } }";
const TABLE_CFG_BASIC_WITH_INDEX: &str =
    "{ table: { mode: basic, index_mode: always, header_on_separator: false } }";

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
