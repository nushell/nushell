use nu_color_config::StyleComputer;
use nu_protocol::{Config, Signals, Span, TableIndent, TableIndexMode, TableMode};

use crate::{common::INDEX_COLUMN_NAME, NuTable};

mod collapse;
mod expanded;
mod general;

pub use collapse::CollapsedTable;
pub use expanded::ExpandedTable;
pub use general::JustTable;

pub struct TableOutput {
    pub table: NuTable,
    pub with_header: bool,
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
    signals: &'a Signals,
    config: &'a Config,
    style_computer: &'a StyleComputer<'a>,
    span: Span,
    width: usize,
    indent: TableIndent,
    mode: TableMode,
    index_offset: usize,
    index_remove: bool,
}

impl<'a> TableOpts<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: &'a Config,
        style_computer: &'a StyleComputer<'a>,
        signals: &'a Signals,
        span: Span,
        width: usize,
        indent: TableIndent,
        mode: TableMode,
        index_offset: usize,
        index_remove: bool,
    ) -> Self {
        Self {
            signals,
            config,
            style_computer,
            span,
            indent,
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
