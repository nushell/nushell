mod nu_protocol_table;
mod table;
mod table_theme;
mod textstyle;

pub use nu_protocol_table::NuTable;
pub use table::{Alignments, Table};
pub use table_theme::TableTheme;
pub use textstyle::{Alignment, TextStyle};

use tabled::{Padding, Style, Width};

pub fn string_width(text: &str) -> usize {
    tabled::papergrid::util::string_width_multiline_tab(text, 4)
}

pub fn wrap_string(text: &str, width: usize) -> String {
    // well... it's not effitient to build a table to wrap a string,
    // but ... it's better than a copy paste
    tabled::builder::Builder::from_iter([[text]])
        .build()
        .with(Padding::zero())
        .with(Style::empty())
        .with(Width::wrap(width))
        .to_string()
}

pub fn string_truncate(text: &str, width: usize) -> String {
    // todo: change me...

    match text.lines().next() {
        Some(first_line) => tabled::builder::Builder::from_iter([[first_line]])
            .build()
            .with(tabled::Style::empty())
            .with(tabled::Padding::zero())
            .with(tabled::Width::truncate(width))
            .to_string(),
        None => String::new(),
    }
}

pub fn string_wrap(text: &str, width: usize) -> String {
    // todo: change me...

    if text.is_empty() {
        return String::new();
    }

    tabled::builder::Builder::from_iter([[text]])
        .build()
        .with(tabled::Style::empty())
        .with(tabled::Padding::zero())
        .with(tabled::Width::wrap(width))
        .to_string()
}
