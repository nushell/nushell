#![doc = include_str!("../README.md")]

mod table;
mod table_theme;
mod types;
mod unstructured_table;
mod util;

pub mod common;

pub use common::{StringResult, TableResult};
pub use nu_color_config::TextStyle;
pub use table::{NuRecords, NuRecordsValue, NuTable};
pub use table_theme::TableTheme;
pub use types::{CollapsedTable, ExpandedTable, JustTable, TableOpts, TableOutput};
pub use unstructured_table::UnstructuredTable;
pub use util::*;
