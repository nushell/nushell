mod common;

use common::{create_row as row, VecCells};
use nu_table::{TableConfig, TableTheme as theme};

#[test]
fn test_rounded() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::rounded()),
        ""
    );
}

#[test]
fn test_basic() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::basic()),
        ""
    );
}

#[test]
fn test_reinforced() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::reinforced()),
        ""
    );
}

#[test]
fn test_compact() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::compact()),
        concat!(
            "───┬───┬───┬───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┼───┼───┼───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┴───┴───┴───",
        )
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::compact()),
        concat!(
            "───┬───┬───┬───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┼───┼───┼───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┴───┴───┴───",
        )
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::compact()),
        concat!("───┬───┬───┬───\n", " 0 │ 1 │ 2 │ 3 \n", "───┴───┴───┴───",)
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::compact()),
        concat!("───┬───┬───┬───\n", " 0 │ 1 │ 2 │ 3 \n", "───┴───┴───┴───",)
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::compact()),
        concat!(
            "───┬───┬───┬───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┴───┴───┴───",
        )
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::compact()),
        ""
    );
}

#[test]
fn test_compact_double() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::compact_double()),
        concat!(
            "═══╦═══╦═══╦═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╬═══╬═══╬═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╩═══╩═══╩═══",
        )
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::compact_double()),
        concat!(
            "═══╦═══╦═══╦═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╬═══╬═══╬═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╩═══╩═══╩═══",
        )
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::compact_double()),
        concat!("═══╦═══╦═══╦═══\n", " 0 ║ 1 ║ 2 ║ 3 \n", "═══╩═══╩═══╩═══",)
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::compact_double()),
        concat!("═══╦═══╦═══╦═══\n", " 0 ║ 1 ║ 2 ║ 3 \n", "═══╩═══╩═══╩═══",)
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::compact_double()),
        concat!(
            "═══╦═══╦═══╦═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╩═══╩═══╩═══",
        )
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::compact_double()),
        ""
    );
}

#[test]
fn test_heavy() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┣━━━╋━━━╋━━━╋━━━┫\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┣━━━╋━━━╋━━━╋━━━┫\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::heavy()),
        ""
    );
}

#[test]
fn test_light() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::light()),
        concat!(
            " 0   1   2   3 \n",
            "───────────────\n",
            " 0   1   2   3 \n",
            " 0   1   2   3 ",
        )
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::light()),
        concat!(" 0   1   2   3 \n", "───────────────\n", " 0   1   2   3 ")
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::light()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::light()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::light()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::light()),
        ""
    );
}

#[test]
fn test_none() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::none()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::none()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::none()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::none()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::none()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::none()),
        ""
    );
}

#[test]
fn test_thin() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::thin()),
        ""
    );
}

#[test]
fn test_with_love() {
    assert_eq!(
        create_table(vec![row(4); 3], true, theme::with_love()),
        concat!(
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
        )
    );

    assert_eq!(
        create_table(vec![row(4); 2], true, theme::with_love()),
        concat!(
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
        )
    );

    assert_eq!(
        create_table(vec![row(4); 1], true, theme::with_love()),
        concat!("❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n", " 0 ❤ 1 ❤ 2 ❤ 3 \n", "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",)
    );

    assert_eq!(
        create_table(vec![row(4); 1], false, theme::with_love()),
        concat!("❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n", " 0 ❤ 1 ❤ 2 ❤ 3 \n", "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",)
    );

    assert_eq!(
        create_table(vec![row(4); 2], false, theme::with_love()),
        concat!(
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
        )
    );

    assert_eq!(
        create_table_with_size(vec![row(4); 0], (0, 4), true, theme::with_love()),
        ""
    );
}

fn create_table(data: VecCells, with_header: bool, theme: theme) -> String {
    let config = TableConfig::new(theme, with_header, false, false);
    let out = common::create_table(data, config, usize::MAX);

    out.expect("not expected to get None")
}

fn create_table_with_size(
    data: VecCells,
    size: (usize, usize),
    with_header: bool,
    theme: theme,
) -> String {
    let config = TableConfig::new(theme, with_header, false, false);

    let table = nu_table::Table::new(data, size);

    table
        .draw(config, usize::MAX)
        .expect("not expected to get None")
}
