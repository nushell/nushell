use nu_color_config::StyleComputer;
use nu_protocol::{Config, Value};

use crate::UnstructuredTable;

use crate::common::nu_value_to_string_clean;
use crate::{
    common::{get_index_style, load_theme_from_config},
    StringResult, TableOpts,
};

pub struct CollapsedTable;

impl CollapsedTable {
    pub fn build(value: Value, opts: TableOpts<'_>) -> StringResult {
        collapsed_table(value, opts.config, opts.width, opts.style_computer)
    }
}

fn collapsed_table(
    mut value: Value,
    config: &Config,
    term_width: usize,
    style_computer: &StyleComputer,
) -> StringResult {
    colorize_value(&mut value, config, style_computer);

    let theme = load_theme_from_config(config);
    let mut table = UnstructuredTable::new(value, config);
    let is_empty = table.truncate(&theme, term_width);
    if is_empty {
        return Ok(None);
    }

    let indent = (config.table_indent.left, config.table_indent.right);
    let table = table.draw(style_computer, &theme, indent);

    Ok(Some(table))
}

fn colorize_value(value: &mut Value, config: &Config, style_computer: &StyleComputer) {
    match value {
        Value::Record { cols, vals, .. } => {
            for val in vals {
                colorize_value(val, config, style_computer);
            }

            let style = get_index_style(style_computer);
            if let Some(color) = style.color_style {
                for header in cols {
                    *header = color.paint(header.to_owned()).to_string();
                }
            }
        }
        Value::List { vals, .. } => {
            for val in vals {
                colorize_value(val, config, style_computer);
            }
        }
        value => {
            let (text, style) = nu_value_to_string_clean(value, config, style_computer);
            if let Some(color) = style.color_style {
                let text = color.paint(text).to_string();
                let span = value.span();
                *value = Value::string(text, span);
            }
        }
    }
}
