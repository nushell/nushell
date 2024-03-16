use crate::{
    clean_charset, colorize_space_str, string_wrap, NuTableConfig, TableOutput, TableTheme,
};
use nu_color_config::{Alignment, StyleComputer, TextStyle};
use nu_protocol::{Config, FooterMode, ShellError, Span, TableMode, TrimStrategy, Value};

pub type NuText = (String, TextStyle);
pub type TableResult = Result<Option<TableOutput>, ShellError>;
pub type StringResult = Result<Option<String>, ShellError>;

pub const INDEX_COLUMN_NAME: &str = "index";

pub fn create_nu_table_config(
    config: &Config,
    comp: &StyleComputer,
    out: &TableOutput,
    expand: bool,
    mode: TableMode,
) -> NuTableConfig {
    NuTableConfig {
        theme: load_theme(mode),
        with_footer: with_footer(config, out.with_header, out.table.count_rows()),
        with_index: out.with_index,
        with_header: out.with_header,
        split_color: Some(lookup_separator_color(comp)),
        trim: config.trim_strategy.clone(),
        header_on_border: config.table_move_header,
        expand,
    }
}

pub fn nu_value_to_string_colored(val: &Value, cfg: &Config, style: &StyleComputer) -> String {
    let (mut text, value_style) = nu_value_to_string(val, cfg, style);
    if let Some(color) = value_style.color_style {
        text = color.paint(text).to_string();
    }

    if matches!(val, Value::String { .. }) {
        text = clean_charset(&text);
        colorize_space_str(&mut text, style);
    }

    text
}

pub fn nu_value_to_string(val: &Value, cfg: &Config, style: &StyleComputer) -> NuText {
    let float_precision = cfg.float_precision as usize;
    let text = val.to_abbreviated_string(cfg);
    make_styled_string(style, text, Some(val), float_precision)
}

pub fn nu_value_to_string_clean(val: &Value, cfg: &Config, style_comp: &StyleComputer) -> NuText {
    let (text, style) = nu_value_to_string(val, cfg, style_comp);
    let mut text = clean_charset(&text);
    colorize_space_str(&mut text, style_comp);

    (text, style)
}

pub fn error_sign(style_computer: &StyleComputer) -> (String, TextStyle) {
    make_styled_string(style_computer, String::from("❎"), None, 0)
}

pub fn wrap_text(text: &str, width: usize, config: &Config) -> String {
    string_wrap(text, width, is_cfg_trim_keep_words(config))
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

pub fn get_empty_style(style_computer: &StyleComputer) -> NuText {
    (
        String::from("❎"),
        TextStyle::with_style(
            Alignment::Right,
            style_computer.compute("empty", &Value::nothing(Span::unknown())),
        ),
    )
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

fn is_cfg_trim_keep_words(config: &Config) -> bool {
    matches!(
        config.trim_strategy,
        TrimStrategy::Wrap {
            try_to_keep_words: true
        }
    )
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
    }
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
