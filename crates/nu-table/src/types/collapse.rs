use crate::{
    common::{get_index_style, load_theme, nu_value_to_string_clean},
    StringResult, TableOpts, UnstructuredTable,
};
use nu_color_config::StyleComputer;
use nu_protocol::{Config, Record, TableMode, Value};
use nu_utils::SharedCow;

pub struct CollapsedTable;

impl CollapsedTable {
    pub fn build(value: Value, opts: TableOpts<'_>) -> StringResult {
        collapsed_table(
            value,
            opts.config,
            opts.width,
            opts.style_computer,
            opts.mode,
        )
    }
}

fn collapsed_table(
    mut value: Value,
    config: &Config,
    term_width: usize,
    style_computer: &StyleComputer,
    mode: TableMode,
) -> StringResult {
    colorize_value(&mut value, config, style_computer);

    let theme = load_theme(mode);
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
        Value::Record { ref mut val, .. } => {
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

                        if let Some(color) = style.color_style {
                            header = color.paint(header).to_string();
                        }
                        (header, val)
                    })
                    .collect::<Record>(),
            );
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
