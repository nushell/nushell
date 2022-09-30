use std::collections::HashMap;

use nu_protocol::Config;
use nu_table::{Alignments, Table, TableTheme as theme, TextStyle};
use tabled::papergrid::records::{cell_info::CellInfo, tcell::TCell};

#[test]
fn test_rounded() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::rounded()),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, false, theme::rounded()), "");
}

#[test]
fn test_basic() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::basic()),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, false, theme::basic()), "");
}

#[test]
fn test_reinforced() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::reinforced()),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 0], 2, false, theme::reinforced()),
        ""
    );
}

#[test]
fn test_compact() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::compact()),
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
        draw_table(vec![row(4); 2], 4, true, theme::compact()),
        concat!(
            "───┬───┬───┬───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┼───┼───┼───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┴───┴───┴───",
        )
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::compact()),
        concat!("───┬───┬───┬───\n", " 0 │ 1 │ 2 │ 3 \n", "───┴───┴───┴───",)
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::compact()),
        concat!("───┬───┬───┬───\n", " 0 │ 1 │ 2 │ 3 \n", "───┴───┴───┴───",)
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::compact()),
        concat!(
            "───┬───┬───┬───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┴───┴───┴───",
        )
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, false, theme::compact()), "");
}

#[test]
fn test_compact_double() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::compact_double()),
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
        draw_table(vec![row(4); 2], 4, true, theme::compact_double()),
        concat!(
            "═══╦═══╦═══╦═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╬═══╬═══╬═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╩═══╩═══╩═══",
        )
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::compact_double()),
        concat!("═══╦═══╦═══╦═══\n", " 0 ║ 1 ║ 2 ║ 3 \n", "═══╩═══╩═══╩═══",)
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::compact_double()),
        concat!("═══╦═══╦═══╦═══\n", " 0 ║ 1 ║ 2 ║ 3 \n", "═══╩═══╩═══╩═══",)
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::compact_double()),
        concat!(
            "═══╦═══╦═══╦═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╩═══╩═══╩═══",
        )
    );

    assert_eq!(
        draw_table(vec![row(4); 0], 4, false, theme::compact_double()),
        ""
    );
}

#[test]
fn test_heavy() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┣━━━╋━━━╋━━━╋━━━┫\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┣━━━╋━━━╋━━━╋━━━┫\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::heavy()),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, false, theme::heavy()), "");
}

#[test]
fn test_light() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::light()),
        concat!(
            " 0   1   2   3 \n",
            "───────────────\n",
            " 0   1   2   3 \n",
            " 0   1   2   3 ",
        )
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::light()),
        concat!(" 0   1   2   3 \n", "───────────────\n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::light()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::light()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::light()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, true, theme::light()), "");
}

#[test]
fn test_none() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::none()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::none()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::none()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::none()),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::none()),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, true, theme::none()), "");
}

#[test]
fn test_thin() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, true, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::thin()),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, true, theme::thin()), "");
}

#[test]
fn test_with_love() {
    assert_eq!(
        draw_table(vec![row(4); 3], 4, true, theme::with_love()),
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
        draw_table(vec![row(4); 2], 4, true, theme::with_love()),
        concat!(
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
        )
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, true, theme::with_love()),
        concat!("❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n", " 0 ❤ 1 ❤ 2 ❤ 3 \n", "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",)
    );

    assert_eq!(
        draw_table(vec![row(4); 1], 4, false, theme::with_love()),
        concat!("❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n", " 0 ❤ 1 ❤ 2 ❤ 3 \n", "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",)
    );

    assert_eq!(
        draw_table(vec![row(4); 2], 4, false, theme::with_love()),
        concat!(
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
        )
    );

    assert_eq!(draw_table(vec![row(4); 0], 4, true, theme::with_love()), "");
}

fn draw_table(
    data: Vec<Vec<TCell<CellInfo<'static>, TextStyle>>>,
    count_columns: usize,
    with_header: bool,
    theme: theme,
) -> String {
    let size = (data.len(), count_columns);
    let table = Table::new(data, size, usize::MAX, with_header, false);

    let cfg = Config::default();
    let styles = HashMap::default();
    let alignments = Alignments::default();
    table
        .draw_table(&cfg, &styles, alignments, &theme, std::usize::MAX)
        .expect("Unexpectdly got no table")
}

fn row(count_columns: usize) -> Vec<TCell<CellInfo<'static>, TextStyle>> {
    let mut row = Vec::with_capacity(count_columns);

    for i in 0..count_columns {
        row.push(Table::create_cell(i.to_string(), TextStyle::default()));
    }

    row
}
