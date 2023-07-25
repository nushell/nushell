mod collapse;
mod expanded;
mod general;

use std::sync::{atomic::AtomicBool, Arc};

pub use collapse::CollapsedTable;
pub use expanded::ExpandedTable;
pub use general::JustTable;
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Span};

use crate::NuTable;

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
    row_offset: usize,
    width: usize,
}

impl<'a> TableOpts<'a> {
    pub fn new(
        config: &'a Config,
        style_computer: &'a StyleComputer<'a>,
        ctrlc: Option<Arc<AtomicBool>>,
        span: Span,
        row_offset: usize,
        available_width: usize,
    ) -> Self {
        Self {
            ctrlc,
            config,
            style_computer,
            span,
            row_offset,
            width: available_width,
        }
    }
}
