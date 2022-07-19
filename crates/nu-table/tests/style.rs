use std::collections::HashMap;

use nu_protocol::Config;
use nu_table::{Alignments, StyledString, Table, TableTheme as theme, TextStyle};

#[test]
fn test_rounded() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::rounded())),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::rounded())),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::rounded())),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::rounded())),
        "╭───┬───┬───┬───╮\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ╰───┴───┴───┴───╯"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::rounded())),
        ""
    );
}

#[test]
fn test_basic() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::basic())),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::basic())),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::basic())),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::basic())),
        "+---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+\n\
         | 0 | 1 | 2 | 3 |\n\
         +---+---+---+---+"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::basic())),
        ""
    );
}

#[test]
fn test_reinforced() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::reinforced())),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::reinforced())),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::reinforced())),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::reinforced())),
        "┏───┬───┬───┬───┓\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ┗───┴───┴───┴───┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::reinforced())),
        ""
    );
}

#[test]
fn test_compact() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::compact())),
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
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::compact())),
        concat!(
            "───┬───┬───┬───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┼───┼───┼───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┴───┴───┴───",
        )
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::compact())),
        concat!("───┬───┬───┬───\n", " 0 │ 1 │ 2 │ 3 \n", "───┴───┴───┴───",)
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::compact())),
        concat!(
            "───┬───┬───┬───\n",
            " 0 │ 1 │ 2 │ 3 \n",
            " 0 │ 1 │ 2 │ 3 \n",
            "───┴───┴───┴───",
        )
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::compact())),
        ""
    );
}

#[test]
fn test_compact_double() {
    assert_eq!(
        draw_table(&Table::new(
            row(4),
            vec![row(4); 2],
            theme::compact_double()
        )),
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
        draw_table(&Table::new(
            row(4),
            vec![row(4); 1],
            theme::compact_double()
        )),
        concat!(
            "═══╦═══╦═══╦═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╬═══╬═══╬═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╩═══╩═══╩═══",
        )
    );

    assert_eq!(
        draw_table(&Table::new(
            row(4),
            vec![row(4); 0],
            theme::compact_double()
        )),
        concat!("═══╦═══╦═══╦═══\n", " 0 ║ 1 ║ 2 ║ 3 \n", "═══╩═══╩═══╩═══",)
    );

    assert_eq!(
        draw_table(&Table::new(
            row(0),
            vec![row(4); 2],
            theme::compact_double()
        )),
        concat!(
            "═══╦═══╦═══╦═══\n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            " 0 ║ 1 ║ 2 ║ 3 \n",
            "═══╩═══╩═══╩═══",
        )
    );

    assert_eq!(
        draw_table(&Table::new(
            row(0),
            vec![row(0); 0],
            theme::compact_double()
        )),
        ""
    );
}

#[test]
fn test_heavy() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::heavy())),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┣━━━╋━━━╋━━━╋━━━┫\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::heavy())),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┣━━━╋━━━╋━━━╋━━━┫\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::heavy())),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::heavy())),
        "┏━━━┳━━━┳━━━┳━━━┓\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┃ 0 ┃ 1 ┃ 2 ┃ 3 ┃\n\
         ┗━━━┻━━━┻━━━┻━━━┛"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::heavy())),
        ""
    );
}

#[test]
fn test_light() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::light())),
        concat!(
            " 0   1   2   3 \n",
            "───────────────\n",
            " 0   1   2   3 \n",
            " 0   1   2   3 ",
        )
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::light())),
        concat!(" 0   1   2   3 \n", "───────────────\n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::light())),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::light())),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::light())),
        ""
    );
}

#[test]
fn test_none() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::none())),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::none())),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::none())),
        concat!(" 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::none())),
        concat!(" 0   1   2   3 \n", " 0   1   2   3 ")
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::none())),
        ""
    );
}

#[test]
fn test_thin() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::thin())),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::thin())),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::thin())),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::thin())),
        "┌───┬───┬───┬───┐\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         ├───┼───┼───┼───┤\n\
         │ 0 │ 1 │ 2 │ 3 │\n\
         └───┴───┴───┴───┘"
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::thin())),
        ""
    );
}

#[test]
fn test_with_love() {
    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 2], theme::with_love())),
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
        draw_table(&Table::new(row(4), vec![row(4); 1], theme::with_love())),
        concat!(
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
        )
    );

    assert_eq!(
        draw_table(&Table::new(row(4), vec![row(4); 0], theme::with_love())),
        concat!("❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n", " 0 ❤ 1 ❤ 2 ❤ 3 \n", "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",)
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(4); 2], theme::with_love())),
        concat!(
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤\n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            " 0 ❤ 1 ❤ 2 ❤ 3 \n",
            "❤❤❤❤❤❤❤❤❤❤❤❤❤❤❤",
        )
    );

    assert_eq!(
        draw_table(&Table::new(row(0), vec![row(0); 0], theme::with_love())),
        ""
    );
}

fn draw_table(table: &Table) -> String {
    let cfg = Config::default();
    let styles = HashMap::default();
    let alignments = Alignments::default();
    table
        .draw_table(&cfg, &styles, alignments, std::usize::MAX)
        .expect("Unexpectdly got no table")
}

fn row(count_columns: usize) -> Vec<StyledString> {
    let mut row = Vec::with_capacity(count_columns);

    for i in 0..count_columns {
        row.push(StyledString::new(i.to_string(), TextStyle::default()));
    }

    row
}
