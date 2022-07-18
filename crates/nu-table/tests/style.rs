use std::collections::HashMap;

use nu_protocol::Config;
use nu_table::{Alignments, StyledString, Table, TableTheme, TextStyle};

#[test]
fn test_rounded_style() {
    let headers = vec![no_style_str("Hello"), no_style_str("World")];
    let data = vec![vec![no_style_str("1"), no_style_str("2")]];

    let table = Table::new(headers, data.clone(), TableTheme::rounded());
    let table = table.draw_table(
        &Config::default(),
        &HashMap::default(),
        Alignments::default(),
        std::usize::MAX,
    );

    assert_eq!(table.as_deref(), Some("╭───────┬───────╮\n│ Hello │ World │\n├───────┼───────┤\n│ 1     │ 2     │\n╰───────┴───────╯"));

    let table = Table::new(Vec::new(), data, TableTheme::rounded());
    let table = table.draw_table(
        &Config::default(),
        &HashMap::default(),
        Alignments::default(),
        std::usize::MAX,
    );

    assert_eq!(table.as_deref(), Some("╭───┬───╮\n│ 1 │ 2 │\n╰───┴───╯"));
}

fn no_style_str(text: &str) -> StyledString {
    StyledString::new(text.to_owned(), TextStyle::default())
}
