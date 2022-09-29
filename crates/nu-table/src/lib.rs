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
