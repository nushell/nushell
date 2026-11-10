use nu_ansi_term::Style;
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Value};
use nu_utils::SharedCow;

use crate::{
    StringResult, TableOpts, UnstructuredTable,
    common::{INDEX_COLUMN_NAME, get_index_style, load_theme, nu_value_to_string_clean},
};

use super::{get_table_row_index, has_index};

pub struct CollapsedTable;

impl CollapsedTable {
    pub fn build(value: Value, opts: TableOpts<'_>) -> StringResult {
        collapsed_table(value, opts)
    }
}

fn collapsed_table(mut value: Value, opts: TableOpts<'_>) -> StringResult {
    colorize_value(&mut value, opts.config, &opts.style_computer);
    insert_index(&mut value, &opts);

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
    match value {
        Value::Record { val, .. } => {
            let style = get_index_style(style_computer);
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

/// Prepends a `#` index column to each record in a homogeneous list when the
/// user explicitly passed `--index`. The index value comes from
/// `get_table_row_index`, which reads an existing `index` column or falls back
/// to the row number. That `index` column is dropped from the output so its
/// values appear only in the `#` column.
///
/// Skips lists whose first item is not a record (plain values, nested lists)
/// and lists where records have different columns (heterogeneous), since the
/// collapsed renderer draws those as key/value rows where a `#` field would
/// be misleading.
fn insert_index(value: &mut Value, opts: &TableOpts<'_>) {
    if !opts.index_explicit {
        return;
    }

    let span = value.span();
    let Value::List { vals, .. } = value else {
        return;
    };

    // Only handle homogeneous record lists.
    let first_columns: Vec<String> = match vals.first() {
        Some(Value::Record { val, .. }) => val.columns().map(String::from).collect(),
        _ => return,
    };

    if !has_index(opts, &first_columns) {
        return;
    }

    // Check that all records share the same columns (homogeneous).
    let homogeneous = vals.iter().all(|v| match v {
        Value::Record { val, .. } => {
            val.columns().count() == first_columns.len()
                && val.columns().zip(first_columns.iter()).all(|(a, b)| a == b)
        }
        _ => false,
    });
    if !homogeneous {
        return;
    }

    let index_style = get_index_style(&opts.style_computer);

    for (row, item) in vals.to_mut().iter_mut().enumerate() {
        let index_text = get_table_row_index(item, opts.config, row, opts.index_offset);
        let styled_index =
            colorize_text(&index_text, index_style.color_style).unwrap_or(index_text);

        if let Value::Record { val, .. } = item {
            let old_record = std::mem::take(val);
            let mut new_record = nu_protocol::Record::with_capacity(old_record.len() + 1);
            new_record.push("#", Value::string(styled_index, span));
            for (col, cell) in old_record.into_owned().into_iter() {
                if col != INDEX_COLUMN_NAME {
                    new_record.push(col, cell);
                }
            }
            *val = SharedCow::new(new_record);
        }
    }
}
