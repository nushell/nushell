use crate::{TableOutput, TableTheme, clean_charset, colorize_space_str, string_wrap};
use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_protocol::{Config, FooterMode, ShellError, Span, TableMode, TrimStrategy, Value};
use nu_utils::terminal_size;

pub type NuText = (String, TextStyle);
pub type TableResult = Result<Option<TableOutput>, ShellError>;
pub type StringResult = Result<Option<String>, ShellError>;

pub const INDEX_COLUMN_NAME: &str = "index";

pub fn configure_table(
    out: &mut TableOutput,
    config: &Config,
    comp: &StyleComputer,
    mode: TableMode,
) {
    let with_footer = is_footer_needed(config, out);
    let theme = load_theme(mode);

    out.table.set_theme(theme);
    out.table
        .set_structure(out.with_index, out.with_header, with_footer);
    out.table.set_trim(config.table.trim.clone());
    out.table
        .set_border_header(config.table.header_on_separator);
    out.table.set_border_color(lookup_separator_color(comp));
}

fn is_footer_needed(config: &Config, out: &TableOutput) -> bool {
    let mut count_rows = out.table.count_rows();
    if config.table.footer_inheritance {
        count_rows = out.count_rows;
    }

    with_footer(config, out.with_header, count_rows)
}

pub fn nu_value_to_string_colored(val: &Value, cfg: &Config, comp: &StyleComputer) -> String {
    let (mut text, style) = nu_value_to_string(val, cfg, comp);

    let is_string = matches!(val, Value::String { .. });
    if is_string {
        text = clean_charset(&text);
    }

    if let Some(color) = style.color_style {
        text = color.paint(text).to_string();
    }

    if is_string {
        colorize_space_str(&mut text, comp);
    }

    text
}

pub fn nu_value_to_string(val: &Value, cfg: &Config, style: &StyleComputer) -> NuText {
    let float_precision = cfg.float_precision as usize;
    let text = val.to_abbreviated_string(cfg);
    make_styled_value(text, val, float_precision, style)
}

// todo: Expose a method which returns just style

pub fn nu_value_to_string_clean(val: &Value, cfg: &Config, style_comp: &StyleComputer) -> NuText {
    let (text, style) = nu_value_to_string(val, cfg, style_comp);
    let mut text = clean_charset(&text);
    colorize_space_str(&mut text, style_comp);

    (text, style)
}

pub fn error_sign(text: String, style_computer: &StyleComputer) -> (String, TextStyle) {
    // Though holes are not the same as null, the closure for "empty" is passed a null anyway.

    let style = style_computer.compute("empty", &Value::nothing(Span::unknown()));
    (text, TextStyle::with_style(Alignment::Center, style))
}

pub fn wrap_text(text: &str, width: usize, config: &Config) -> String {
    let keep_words = config.table.trim == TrimStrategy::wrap(true);
    string_wrap(text, width, keep_words)
}

pub fn get_header_style(style_computer: &StyleComputer) -> TextStyle {
    TextStyle::with_style(
        Alignment::Center,
        style_computer.compute("header", &Value::string("", Span::unknown())),
    )
}

pub fn get_index_style(style_computer: &StyleComputer) -> TextStyle {
    TextStyle::with_style(
        Alignment::Right,
        style_computer.compute("row_index", &Value::string("", Span::unknown())),
    )
}

pub fn get_leading_trailing_space_style(style_computer: &StyleComputer) -> TextStyle {
    TextStyle::with_style(
        Alignment::Right,
        style_computer.compute(
            "leading_trailing_space_bg",
            &Value::string("", Span::unknown()),
        ),
    )
}

pub fn get_value_style(value: &Value, config: &Config, style_computer: &StyleComputer) -> NuText {
    match value {
        // Float precision is required here.
        Value::Float { val, .. } => (
            format!("{:.prec$}", val, prec = config.float_precision as usize),
            style_computer.style_primitive(value),
        ),
        _ => (
            value.to_abbreviated_string(config),
            style_computer.style_primitive(value),
        ),
    }
}

pub fn get_empty_style(text: String, style_computer: &StyleComputer) -> NuText {
    (
        text,
        TextStyle::with_style(
            Alignment::Right,
            style_computer.compute("empty", &Value::nothing(Span::unknown())),
        ),
    )
}

fn make_styled_value(
    text: String,
    value: &Value,
    float_precision: usize,
    style_computer: &StyleComputer,
) -> NuText {
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

fn convert_with_precision(val: &str, precision: usize) -> Result<String, ShellError> {
    // vall will always be a f64 so convert it with precision formatting
    let val_float = match val.trim().parse::<f64>() {
        Ok(f) => f,
        Err(e) => {
            return Err(ShellError::GenericError {
                error: format!("error converting string [{}] to f64", &val),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
            });
        }
    };
    Ok(format!("{val_float:.precision$}"))
}

pub fn load_theme(mode: TableMode) -> TableTheme {
    match mode {
        TableMode::Basic => TableTheme::basic(),
        TableMode::Thin => TableTheme::thin(),
        TableMode::Light => TableTheme::light(),
        TableMode::Compact => TableTheme::compact(),
        TableMode::WithLove => TableTheme::with_love(),
        TableMode::CompactDouble => TableTheme::compact_double(),
        TableMode::Rounded => TableTheme::rounded(),
        TableMode::Reinforced => TableTheme::reinforced(),
        TableMode::Heavy => TableTheme::heavy(),
        TableMode::None => TableTheme::none(),
        TableMode::Psql => TableTheme::psql(),
        TableMode::Markdown => TableTheme::markdown(),
        TableMode::Dots => TableTheme::dots(),
        TableMode::Restructured => TableTheme::restructured(),
        TableMode::AsciiRounded => TableTheme::ascii_rounded(),
        TableMode::BasicCompact => TableTheme::basic_compact(),
        TableMode::Single => TableTheme::single(),
        TableMode::Double => TableTheme::double(),
    }
}

fn lookup_separator_color(style_computer: &StyleComputer) -> nu_ansi_term::Style {
    style_computer.compute("separator", &Value::nothing(Span::unknown()))
}

fn with_footer(config: &Config, with_header: bool, count_records: usize) -> bool {
    with_header && need_footer(config, count_records as u64)
}

fn need_footer(config: &Config, count_records: u64) -> bool {
    match config.footer_mode {
        // Only show the footer if there are more than RowCount rows
        FooterMode::RowCount(limit) => count_records > limit,
        // Always show the footer
        FooterMode::Always => true,
        // Never show the footer
        FooterMode::Never => false,
        // Calculate the screen height and row count, if screen height is larger than row count, don't show footer
        FooterMode::Auto => {
            let (_width, height) = match terminal_size() {
                Ok((w, h)) => (w as u64, h as u64),
                _ => (0, 0),
            };
            height <= count_records
        }
    }
}

pub fn check_value(value: &Value) -> Result<(), ShellError> {
    match value {
        Value::Error { error, .. } => Err(*error.clone()),
        _ => Ok(()),
    }
}
