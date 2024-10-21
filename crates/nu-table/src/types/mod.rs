use terminal_size::{terminal_size, Height, Width};

use crate::{common::INDEX_COLUMN_NAME, NuTable};
use nu_color_config::StyleComputer;
use nu_protocol::{Config, FooterMode, Signals, Span, TableIndexMode, TableMode};

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
    pub with_footer: bool,
}

impl TableOutput {
    pub fn new(table: NuTable, with_header: bool, with_index: bool, with_footer: bool) -> Self {
        Self {
            table,
            with_header,
            with_index,
            with_footer,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableOpts<'a> {
    signals: &'a Signals,
    config: &'a Config,
    style_computer: &'a StyleComputer<'a>,
    span: Span,
    width: usize,
    indent: (usize, usize),
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
        indent: (usize, usize),
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

fn has_footer(opts: &TableOpts<'_>, count_records: u64) -> bool {
    match opts.config.footer_mode {
        // Only show the footer if there are more than RowCount rows
        FooterMode::RowCount(limit) => count_records > limit,
        // Always show the footer
        FooterMode::Always => true,
        // Never show the footer
        FooterMode::Never => false,
        // Calculate the screen height and row count, if screen height is larger than row count, don't show footer
        FooterMode::Auto => {
            let (_width, height) = match terminal_size() {
                Some((w, h)) => (Width(w.0).0 as u64, Height(h.0).0 as u64),
                None => (Width(0).0 as u64, Height(0).0 as u64),
            };

            height <= count_records
        }
    }
}
