use super::prelude::*;
use crate as nu_protocol;
use crate::ShellError;

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableMode {
    Basic,
    Thin,
    Light,
    Compact,
    WithLove,
    CompactDouble,
    #[default]
    Rounded,
    Reinforced,
    Heavy,
    None,
    Psql,
    Markdown,
    Dots,
    Restructured,
    AsciiRounded,
    BasicCompact,
}

impl FromStr for TableMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "basic" => Ok(Self::Basic),
            "thin" => Ok(Self::Thin),
            "light" => Ok(Self::Light),
            "compact" => Ok(Self::Compact),
            "with_love" => Ok(Self::WithLove),
            "compact_double" => Ok(Self::CompactDouble),
            "default" => Ok(TableMode::default()),
            "rounded" => Ok(Self::Rounded),
            "reinforced" => Ok(Self::Reinforced),
            "heavy" => Ok(Self::Heavy),
            "none" => Ok(Self::None),
            "psql" => Ok(Self::Psql),
            "markdown" => Ok(Self::Markdown),
            "dots" => Ok(Self::Dots),
            "restructured" => Ok(Self::Restructured),
            "ascii_rounded" => Ok(Self::AsciiRounded),
            "basic_compact" => Ok(Self::BasicCompact),
            _ => Err("expected either 'basic', 'thin', 'light', 'compact', 'with_love', 'compact_double', 'rounded', 'reinforced', 'heavy', 'none', 'psql', 'markdown', 'dots', 'restructured', 'ascii_rounded', or 'basic_compact'"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FooterMode {
    /// Never show the footer
    Never,
    /// Always show the footer
    Always,
    /// Only show the footer if there are more than RowCount rows
    RowCount(u64),
    /// Calculate the screen height, calculate row count, if display will be bigger than screen, add the footer
    Auto,
}

impl FromStr for FooterMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "always" => Ok(FooterMode::Always),
            "never" => Ok(FooterMode::Never),
            "auto" => Ok(FooterMode::Auto),
            x => {
                if let Ok(count) = x.parse() {
                    Ok(FooterMode::RowCount(count))
                } else {
                    Err("expected either 'never', 'always', 'auto' or a row count")
                }
            }
        }
    }
}

impl IntoValue for FooterMode {
    fn into_value(self, span: Span) -> Value {
        match self {
            FooterMode::Always => "always".into_value(span),
            FooterMode::Never => "never".into_value(span),
            FooterMode::Auto => "auto".into_value(span),
            FooterMode::RowCount(c) => c.to_string().into_value(span),
        }
    }
}

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableIndexMode {
    /// Always show indexes
    Always,
    /// Never show indexes
    Never,
    /// Show indexes when a table has "index" column
    Auto,
}

impl FromStr for TableIndexMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "always" => Ok(TableIndexMode::Always),
            "never" => Ok(TableIndexMode::Never),
            "auto" => Ok(TableIndexMode::Auto),
            _ => Err("expected either 'never', 'always' or 'auto'"),
        }
    }
}

/// A Table view configuration, for a situation where
/// we need to limit cell width in order to adjust for a terminal size.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrimStrategy {
    /// Wrapping strategy.
    ///
    /// It it's similar to original nu_table, strategy.
    Wrap {
        /// A flag which indicates whether is it necessary to try
        /// to keep word boundaries.
        try_to_keep_words: bool,
    },
    /// Truncating strategy, where we just cut the string.
    /// And append the suffix if applicable.
    Truncate {
        /// Suffix which can be appended to a truncated string after being cut.
        ///
        /// It will be applied only when there's enough room for it.
        /// For example in case where a cell width must be 12 chars, but
        /// the suffix takes 13 chars it won't be used.
        suffix: Option<String>,
    },
}

impl TrimStrategy {
    pub fn wrap(dont_split_words: bool) -> Self {
        Self::Wrap {
            try_to_keep_words: dont_split_words,
        }
    }

    pub fn truncate(suffix: Option<String>) -> Self {
        Self::Truncate { suffix }
    }
}

impl Default for TrimStrategy {
    fn default() -> Self {
        Self::Wrap {
            try_to_keep_words: true,
        }
    }
}

pub(super) fn try_parse_trim_strategy(
    value: &Value,
    errors: &mut Vec<ShellError>,
) -> Result<TrimStrategy, ShellError> {
    let map = value.as_record().map_err(|e| ShellError::GenericError {
        error: "Error while applying config changes".into(),
        msg: "$env.config.table.trim is not a record".into(),
        span: Some(value.span()),
        help: Some("Please consult the documentation for configuring Nushell.".into()),
        inner: vec![e],
    })?;

    let mut methodology = match map.get("methodology") {
        Some(value) => match try_parse_trim_methodology(value) {
            Some(methodology) => methodology,
            None => return Ok(TrimStrategy::default()),
        },
        None => {
            errors.push(ShellError::GenericError {
                error: "Error while applying config changes".into(),
                msg: "$env.config.table.trim.methodology was not provided".into(),
                span: Some(value.span()),
                help: Some("Please consult the documentation for configuring Nushell.".into()),
                inner: vec![],
            });
            return Ok(TrimStrategy::default());
        }
    };

    match &mut methodology {
        TrimStrategy::Wrap { try_to_keep_words } => {
            if let Some(value) = map.get("wrapping_try_keep_words") {
                if let Ok(b) = value.as_bool() {
                    *try_to_keep_words = b;
                } else {
                    errors.push(ShellError::GenericError {
                        error: "Error while applying config changes".into(),
                        msg: "$env.config.table.trim.wrapping_try_keep_words is not a bool".into(),
                        span: Some(value.span()),
                        help: Some(
                            "Please consult the documentation for configuring Nushell.".into(),
                        ),
                        inner: vec![],
                    });
                }
            }
        }
        TrimStrategy::Truncate { suffix } => {
            if let Some(value) = map.get("truncating_suffix") {
                if let Ok(v) = value.coerce_string() {
                    *suffix = Some(v);
                } else {
                    errors.push(ShellError::GenericError {
                        error: "Error while applying config changes".into(),
                        msg: "$env.config.table.trim.truncating_suffix is not a string".into(),
                        span: Some(value.span()),
                        help: Some(
                            "Please consult the documentation for configuring Nushell.".into(),
                        ),
                        inner: vec![],
                    });
                }
            }
        }
    }

    Ok(methodology)
}

fn try_parse_trim_methodology(value: &Value) -> Option<TrimStrategy> {
    if let Ok(value) = value.coerce_str() {
        match value.to_lowercase().as_str() {
        "wrapping" => {
            return Some(TrimStrategy::Wrap {
                try_to_keep_words: false,
            });
        }
        "truncating" => return Some(TrimStrategy::Truncate { suffix: None }),
        _ => eprintln!("unrecognized $config.table.trim.methodology value; expected either 'truncating' or 'wrapping'"),
    }
    } else {
        eprintln!("$env.config.table.trim.methodology is not a string")
    }

    None
}

impl IntoValue for TrimStrategy {
    fn into_value(self, span: Span) -> Value {
        match self {
            TrimStrategy::Wrap { try_to_keep_words } => {
                record! {
                    "methodology" => "wrapping".into_value(span),
                    "wrapping_try_keep_words" => try_to_keep_words.into_value(span),
                }
            }
            TrimStrategy::Truncate { suffix } => {
                record! {
                    "methodology" => "truncating".into_value(span),
                    "truncating_suffix" => suffix.into_value(span),
                }
            }
        }
        .into_value(span)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableIndent {
    pub left: usize,
    pub right: usize,
}

impl IntoValue for TableIndent {
    fn into_value(self, span: Span) -> Value {
        record! {
            "left" => (self.left as i64).into_value(span),
            "right" => (self.right as i64).into_value(span),
        }
        .into_value(span)
    }
}

impl Default for TableIndent {
    fn default() -> Self {
        Self { left: 1, right: 1 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableConfig {
    pub mode: TableMode,
    pub index_mode: TableIndexMode,
    pub show_empty: bool,
    pub padding: TableIndent,
    pub trim: TrimStrategy,
    pub header_on_separator: bool,
    pub abbreviated_row_count: Option<usize>,
}

impl IntoValue for TableConfig {
    fn into_value(self, span: Span) -> Value {
        let abbv_count = self
            .abbreviated_row_count
            .map(|t| t as i64)
            .into_value(span);

        record! {
            "mode" => self.mode.into_value(span),
            "index_mode" => self.index_mode.into_value(span),
            "show_empty" => self.show_empty.into_value(span),
            "padding" => self.padding.into_value(span),
            "trim" => self.trim.into_value(span),
            "header_on_separator" => self.header_on_separator.into_value(span),
            "abbreviated_row_count" => abbv_count,
        }
        .into_value(span)
    }
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            mode: TableMode::Rounded,
            index_mode: TableIndexMode::Always,
            show_empty: true,
            trim: TrimStrategy::default(),
            header_on_separator: false,
            padding: TableIndent::default(),
            abbreviated_row_count: None,
        }
    }
}
