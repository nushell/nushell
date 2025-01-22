use nu_color_config::StyleComputer;
use nu_protocol::{Config, Signals, Span, TableIndexMode, TableMode};

use crate::{common::INDEX_COLUMN_NAME, NuTable};

mod collapse;
mod expanded;
mod general;

pub use collapse::CollapsedTable;
pub use expanded::ExpandedTable;
pub use general::JustTable;

pub struct TableOutput {
    /// A table structure.
    pub table: NuTable,
    /// A flag whether a header was present in the table.
    pub with_header: bool,
    /// A flag whether a index was present in the table.
    pub with_index: bool,
    /// The value may be bigger then table.count_rows(),
    /// Specifically in case of expanded table we collect the whole structure size here.
    pub count_rows: usize,
}

impl TableOutput {
    pub fn new(table: NuTable, with_header: bool, with_index: bool, count_rows: usize) -> Self {
        Self {
            table,
            with_header,
            with_index,
            count_rows,
        }
    }
    pub fn from_table(table: NuTable, with_header: bool, with_index: bool) -> Self {
        let count_rows = table.count_rows();
        Self::new(table, with_header, with_index, count_rows)
    }
}

#[derive(Debug, Clone)]
pub struct TableOpts<'a> {
    pub signals: &'a Signals,
    pub config: &'a Config,
    pub style_computer: std::rc::Rc<StyleComputer<'a>>,
    pub span: Span,
    pub width: usize,
    pub mode: TableMode,
    pub index_offset: usize,
    pub index_remove: bool,
}

impl<'a> TableOpts<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: &'a Config,
        style_computer: StyleComputer<'a>,
        signals: &'a Signals,
        span: Span,
        width: usize,
        mode: TableMode,
        index_offset: usize,
        index_remove: bool,
    ) -> Self {
        let style_computer = std::rc::Rc::new(style_computer);

        Self {
            signals,
            config,
            style_computer,
            span,
            width,
            mode,
            index_offset,
            index_remove,
        }
    }
}

fn has_index(opts: &TableOpts<'_>, headers: &[String]) -> bool {
    let with_index = match opts.config.table.index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

    with_index && !opts.index_remove
}
