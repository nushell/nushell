use nu_color_config::StyleComputer;
use nu_protocol::{Config, Span, Value};

use crate::UnstructuredTable;

use super::{
    clean_charset, general::BuildConfig, get_index_style, load_theme_from_config,
    value_to_styled_string, StringResult,
};

pub struct CollapsedTable;

impl CollapsedTable {
    pub fn build(value: Value, opts: BuildConfig<'_>) -> StringResult {
        collapsed_table(value, opts.config, opts.term_width, opts.style_computer)
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

    let table = table.draw(style_computer, &theme);

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
            let (text, style) = value_to_styled_string(value, config, style_computer);

            let is_string = matches!(value, Value::String { .. });
            if is_string {
                let mut text = clean_charset(&text);
                if let Some(color) = style.color_style {
                    text = color.paint(text).to_string();
                }

                let span = value.span().unwrap_or(Span::unknown());
                *value = Value::string(text, span);
                return;
            }

            if let Some(color) = style.color_style {
                let text = color.paint(text).to_string();
                let span = value.span().unwrap_or(Span::unknown());
                *value = Value::string(text, span);
            }
        }
    }
}
