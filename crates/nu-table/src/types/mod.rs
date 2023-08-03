mod collapse;
mod expanded;
mod general;

use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_protocol::TrimStrategy;
use nu_protocol::{Config, FooterMode, ShellError, Span, Value};
use std::collections::HashMap;

use crate::{string_wrap, NuTable, TableConfig, TableTheme};

pub use collapse::CollapsedTable;
pub use expanded::ExpandedTable;
pub use general::{BuildConfig, JustTable};

pub type NuText = (String, TextStyle);
pub type TableResult = Result<Option<TableOutput>, ShellError>;
pub type StringResult = Result<Option<String>, ShellError>;

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

pub fn value_to_styled_string(val: &Value, cfg: &Config, style: &StyleComputer) -> NuText {
    let float_precision = cfg.float_precision as usize;
    let text = val.into_abbreviated_string(cfg);
    make_styled_string(style, text, Some(val), float_precision)
}

pub fn value_to_clean_styled_string(val: &Value, cfg: &Config, style: &StyleComputer) -> NuText {
    let (text, style) = value_to_styled_string(val, cfg, style);
    let text = clean_charset(&text);
    (text, style)
}

pub fn clean_charset(text: &str) -> String {
    // todo: optimize, I bet it can be done in 1 path
    text.replace('\t', "    ").replace('\r', "")
}

const INDEX_COLUMN_NAME: &str = "index";

fn error_sign(style_computer: &StyleComputer) -> (String, TextStyle) {
    make_styled_string(style_computer, String::from("❎"), None, 0)
}

fn wrap_text(text: &str, width: usize, config: &Config) -> String {
    string_wrap(text, width, is_cfg_trim_keep_words(config))
}

fn make_styled_string(
    style_computer: &StyleComputer,
    text: String,
    value: Option<&Value>, // None represents table holes.
    float_precision: usize,
) -> NuText {
    match value {
        Some(value) => {
            match value {
                Value::Float { .. } => {
                    // set dynamic precision from config
                    let precise_number = match convert_with_precision(&text, float_precision) {
                        Ok(num) => num,
                        Err(e) => e.to_string(),
                    };
                    (precise_number, style_computer.style_primitive(value))
                }
                _ => (text, style_computer.style_primitive(value)),
            }
        }
        None => {
            // Though holes are not the same as null, the closure for "empty" is passed a null anyway.
            (
                text,
                TextStyle::with_style(
                    Alignment::Center,
                    style_computer.compute("empty", &Value::nothing(Span::unknown())),
                ),
            )
        }
    }
}

fn convert_with_precision(val: &str, precision: usize) -> Result<String, ShellError> {
    // vall will always be a f64 so convert it with precision formatting
    let val_float = match val.trim().parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(ShellError::GenericError(
                format!("error converting string [{}] to f64", &val),
                "".to_string(),
                None,
                Some(e.to_string()),
                Vec::new(),
            ));
        }
    };
    Ok(format!("{val_float:.precision$}"))
}

fn is_cfg_trim_keep_words(config: &Config) -> bool {
    matches!(
        config.trim_strategy,
        TrimStrategy::Wrap {
            try_to_keep_words: true
        }
    )
}

fn load_theme_from_config(config: &Config) -> TableTheme {
    match config.table_mode.as_str() {
        "basic" => TableTheme::basic(),
        "thin" => TableTheme::thin(),
        "light" => TableTheme::light(),
        "compact" => TableTheme::compact(),
        "with_love" => TableTheme::with_love(),
        "compact_double" => TableTheme::compact_double(),
        "rounded" => TableTheme::rounded(),
        "reinforced" => TableTheme::reinforced(),
        "heavy" => TableTheme::heavy(),
        "none" => TableTheme::none(),
        _ => TableTheme::rounded(),
    }
}

fn create_table_config(config: &Config, comp: &StyleComputer, out: &TableOutput) -> TableConfig {
    let theme = load_theme_from_config(config);
    let footer = with_footer(config, out.with_header, out.table.count_rows());
    let line_style = lookup_separator_color(comp);
    let trim = config.trim_strategy.clone();

    TableConfig::new()
        .theme(theme)
        .with_footer(footer)
        .with_header(out.with_header)
        .with_index(out.with_index)
        .line_style(line_style)
        .trim(trim)
}

fn lookup_separator_color(style_computer: &StyleComputer) -> nu_ansi_term::Style {
    style_computer.compute("separator", &Value::nothing(Span::unknown()))
}

fn with_footer(config: &Config, with_header: bool, count_records: usize) -> bool {
    with_header && need_footer(config, count_records as u64)
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    matches!(config.footer_mode, FooterMode::RowCount(limit) if count_records > limit)
        || matches!(config.footer_mode, FooterMode::Always)
}

fn set_data_styles(table: &mut NuTable, styles: HashMap<(usize, usize), TextStyle>) {
    for (pos, style) in styles {
        table.set_cell_style(pos, style);
    }
}

fn get_header_style(style_computer: &StyleComputer) -> TextStyle {
    TextStyle::with_style(
        Alignment::Center,
        style_computer.compute("header", &Value::string("", Span::unknown())),
    )
}

fn get_index_style(style_computer: &StyleComputer) -> TextStyle {
    TextStyle::with_style(
        Alignment::Right,
        style_computer.compute("row_index", &Value::string("", Span::unknown())),
    )
}

fn get_value_style(value: &Value, config: &Config, style_computer: &StyleComputer) -> NuText {
    match value {
        // Float precision is required here.
        Value::Float { val, .. } => (
            format!("{:.prec$}", val, prec = config.float_precision as usize),
            style_computer.style_primitive(value),
        ),
        _ => (
            value.into_abbreviated_string(config),
            style_computer.style_primitive(value),
        ),
    }
}

fn get_empty_style(style_computer: &StyleComputer) -> NuText {
    (
        String::from("❎"),
        TextStyle::with_style(
            Alignment::Right,
            style_computer.compute("empty", &Value::nothing(Span::unknown())),
        ),
    )
}
