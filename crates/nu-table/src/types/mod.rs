mod collapse;
mod expanded;
mod general;

pub use collapse::CollapsedTable;
pub use expanded::ExpandedTable;
pub use general::JustTable;

use crate::{common::INDEX_COLUMN_NAME, NuTable};
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Span, TableIndexMode, TableMode};
use std::sync::{atomic::AtomicBool, Arc};

pub struct TableOutput {
    pub table: NuTable,
    pub with_header: bool,
    pub with_index: bool,
}

impl TableOutput {
    pub fn new(table: NuTable, with_header: bool, with_index: bool) -> Self {
        Self {
            table,
            with_header,
            with_index,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableOpts<'a> {
    ctrlc: Option<Arc<AtomicBool>>,
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
        ctrlc: Option<Arc<AtomicBool>>,
        span: Span,
        width: usize,
        indent: (usize, usize),
        mode: TableMode,
        index_offset: usize,
        index_remove: bool,
    ) -> Self {
        Self {
            ctrlc,
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
    let with_index = match opts.config.table_index_mode {
        TableIndexMode::Always => true,
        TableIndexMode::Never => false,
        TableIndexMode::Auto => headers.iter().any(|header| header == INDEX_COLUMN_NAME),
    };

    with_index && !opts.index_remove
}
