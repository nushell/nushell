use nu_ansi_term::Style;
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Record, Value};
use nu_utils::SharedCow;

use crate::{
    StringResult, TableOpts, UnstructuredTable,
    common::{INDEX_COLUMN_NAME, get_index_style, load_theme, nu_value_to_string_clean},
    unstructured_table::is_valid_record,
};

use super::{get_table_row_index, has_index};

pub struct CollapsedTable;

impl CollapsedTable {
    pub fn build(value: Value, opts: TableOpts<'_>) -> StringResult {
        collapsed_table(value, opts)
    }
}

fn collapsed_table(mut value: Value, opts: TableOpts<'_>) -> StringResult {
    let indexes = strip_index_column(&mut value, &opts);
    colorize_value(&mut value, opts.config, &opts.style_computer);
    if let Some(indexes) = indexes {
        prepend_index_column(&mut value, &opts, indexes);
    }

    let mut table = UnstructuredTable::new(value, opts.config);

    let theme = load_theme(opts.mode);
    let is_empty = table.truncate(&theme, opts.width);
    if is_empty {
        return Ok(None);
    }

    let table = table.draw(&theme, &opts.style_computer);

    Ok(Some(table))
}

fn colorize_value(value: &mut Value, config: &Config, style_computer: &StyleComputer) {
    // todo: Remove recursion?

    match value {
        Value::Record { val, .. } => {
            let style = get_index_style(style_computer);
            // Take ownership of the record and reassign to &mut
            // We do this to have owned keys through `.into_iter`
            let record = std::mem::take(val);
            *val = SharedCow::new(
                record
                    .into_owned()
                    .into_iter()
                    .map(|(mut header, mut val)| {
                        colorize_value(&mut val, config, style_computer);
                        header = colorize_text(&header, style.color_style).unwrap_or(header);

                        (header, val)
                    })
                    .collect(),
            );
        }
        Value::List { vals, .. } => {
            for val in vals.to_mut() {
                colorize_value(val, config, style_computer);
            }
        }
        value => {
            let (text, style) = nu_value_to_string_clean(value, config, style_computer);
            if let Some(text) = colorize_text(&text, style.color_style) {
                *value = Value::string(text, value.span());
            }
        }
    }
}

fn colorize_text(text: &str, color: Option<Style>) -> Option<String> {
    if let Some(color) = color
        && !color.is_plain()
    {
        return Some(color.paint(text).to_string());
    }

    None
}

/// Extracts index values from each record in a homogeneous list and removes
/// the `index` column when the user explicitly passed `--index`.
///
/// Returns `Some(Vec<String>)` of per-row index texts when the list qualifies
/// (homogeneous records, `has_index` is true), or `None` to skip indexing.
/// The `index` column is stripped here so that `colorize_value` never sees it,
/// and `prepend_index_column` can later add the styled `#` column.
///
/// Skips lists whose first item is not a record (plain values, nested lists)
/// and lists where records have different columns (heterogeneous), since the
/// collapsed renderer draws those as key/value rows where a `#` field would
/// be misleading.
fn strip_index_column(value: &mut Value, opts: &TableOpts<'_>) -> Option<Vec<String>> {
    if !opts.index_explicit {
        return None;
    }

    let Value::List { vals, .. } = value else {
        return None;
    };

    let first_columns: Vec<String> = match vals.first() {
        Some(Value::Record { val, .. }) => val.columns().map(String::from).collect(),
        _ => return None,
    };

    if !has_index(opts, &first_columns) {
        return None;
    }

    if !is_valid_record(vals) {
        return None;
    }

    let mut indexes = Vec::with_capacity(vals.len());

    for (row, item) in vals.to_mut().iter_mut().enumerate() {
        let index_text = get_table_row_index(item, opts.config, row, opts.index_offset);
        indexes.push(index_text);

        if let Value::Record { val, .. } = item {
            let old_record = std::mem::take(val);
            let mut new_record = Record::with_capacity(old_record.len());
            for (col, cell) in old_record.into_owned().into_iter() {
                if col != INDEX_COLUMN_NAME {
                    new_record.push(col, cell);
                }
            }
            *val = SharedCow::new(new_record);
        }
    }

    Some(indexes)
}

/// Prepends a `#` index column to each record using previously extracted index
/// values. Both the `#` key and the index number are painted with the
/// `row_index` style so they match the default and expanded table modes.
fn prepend_index_column(value: &mut Value, opts: &TableOpts<'_>, indexes: Vec<String>) {
    let span = value.span();
    let index_style = get_index_style(&opts.style_computer);

    let Value::List { vals, .. } = value else {
        return;
    };

    let styled_header = colorize_text("#", index_style.color_style).unwrap_or("#".to_string());

    for (item, index_text) in vals.to_mut().iter_mut().zip(indexes) {
        let styled_index =
            colorize_text(&index_text, index_style.color_style).unwrap_or(index_text);

        if let Value::Record { val, .. } = item {
            let old_record = std::mem::take(val);
            let mut new_record = Record::with_capacity(old_record.len() + 1);
            new_record.push(&styled_header, Value::string(styled_index, span));
            for (col, cell) in old_record.into_owned().into_iter() {
                new_record.push(col, cell);
            }
            *val = SharedCow::new(new_record);
        }
    }
}
