mod nu_protocol_table;
mod table;
mod table_theme;
mod textstyle;

pub use nu_protocol_table::NuTable;
pub use table::{Alignments, Table};
pub use table_theme::TableTheme;
pub use textstyle::{Alignment, TextStyle};

pub fn string_width(text: &str) -> usize {
    tabled::papergrid::util::string_width_multiline_tab(text, 4)
}

pub fn string_truncate(text: &str, width: usize) -> String {
    // todo: change me...

    if text.is_empty() {
        return String::new();
    }
    let first_line = text.lines().next().unwrap();

    tabled::builder::Builder::from_iter([[first_line]])
        .build()
        .with(tabled::Style::empty())
        .with(tabled::Padding::zero())
        .with(tabled::Width::truncate(width))
        .to_string()
}
