use nu_color_config::StyleComputer;
use nu_protocol::{Config, Signals, Span, TableIndexMode, TableMode, Value};

use crate::{NuTable, common::INDEX_COLUMN_NAME};

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
    pub width_priority_columns: Vec<String>,
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
        width_priority_columns: Vec<String>,
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
            width_priority_columns,
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

/// Resolves configured priority column names into concrete table column indexes.
///
/// The returned indexes preserve caller order and contain no duplicates.
pub(crate) fn resolve_priority_columns(
    headers: &[String],
    width_priority_columns: &[String],
    with_index: bool,
) -> Vec<usize> {
    let mut resolved = Vec::new();

    for priority_column in width_priority_columns {
        let mut resolved_index = None;

        if with_index && priority_column == INDEX_COLUMN_NAME {
            resolved_index = Some(0);
        } else {
            let mut table_index = usize::from(with_index);
            for header in headers {
                if header == INDEX_COLUMN_NAME && with_index {
                    continue;
                }

                if header == priority_column {
                    resolved_index = Some(table_index);
                    break;
                }

                table_index += 1;
            }
        }

        if let Some(index) = resolved_index
            && !resolved.contains(&index)
        {
            resolved.push(index);
        }
    }

    resolved
}

/// The index shown for row `row` of a table: the row's `index` column when it
/// has one, its position counted from `offset` otherwise.
pub(crate) fn get_table_row_index(
    item: &Value,
    config: &Config,
    row: usize,
    offset: usize,
) -> String {
    match item {
        Value::Record { val, .. } => val
            .get(INDEX_COLUMN_NAME)
            .map(|value| value.to_expanded_string("", config))
            .unwrap_or_else(|| (row + offset).to_string()),
        _ => (row + offset).to_string(),
    }
}
