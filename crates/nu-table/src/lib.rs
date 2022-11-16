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
