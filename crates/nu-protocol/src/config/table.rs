use super::helper::ReconstructVal;
use crate::{record, Config, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
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

impl ReconstructVal for TableMode {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(
            match self {
                TableMode::Basic => "basic",
                TableMode::Thin => "thin",
                TableMode::Light => "light",
                TableMode::Compact => "compact",
                TableMode::WithLove => "with_love",
                TableMode::CompactDouble => "compact_double",
                TableMode::Rounded => "rounded",
                TableMode::Reinforced => "reinforced",
                TableMode::Heavy => "heavy",
                TableMode::None => "none",
                TableMode::Psql => "psql",
                TableMode::Markdown => "markdown",
                TableMode::Dots => "dots",
                TableMode::Restructured => "restructured",
                TableMode::AsciiRounded => "ascii_rounded",
                TableMode::BasicCompact => "basic_compact",
            },
            span,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

impl ReconstructVal for FooterMode {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(
            match self {
                FooterMode::Always => "always".to_string(),
                FooterMode::Never => "never".to_string(),
                FooterMode::Auto => "auto".to_string(),
                FooterMode::RowCount(c) => c.to_string(),
            },
            span,
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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

impl ReconstructVal for TableIndexMode {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(
            match self {
                TableIndexMode::Always => "always",
                TableIndexMode::Never => "never",
                TableIndexMode::Auto => "auto",
            },
            span,
        )
    }
}

/// A Table view configuration, for a situation where
/// we need to limit cell width in order to adjust for a terminal size.
#[derive(Serialize, Deserialize, Clone, Debug)]
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
        TrimStrategy::Wrap {
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
                if let Ok(v) = value.as_string() {
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
    if let Ok(value) = value.as_string() {
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

pub(super) fn reconstruct_trim_strategy(config: &Config, span: Span) -> Value {
    match &config.trim_strategy {
        TrimStrategy::Wrap { try_to_keep_words } => Value::record(
            record! {
                "methodology" => Value::string("wrapping", span),
                "wrapping_try_keep_words" => Value::bool(*try_to_keep_words, span),
            },
            span,
        ),
        TrimStrategy::Truncate { suffix } => Value::record(
            match suffix {
                Some(s) => record! {
                    "methodology" => Value::string("truncating", span),
                    "truncating_suffix" => Value::string(s.clone(), span),
                },
                None => record! {
                    "methodology" => Value::string("truncating", span),
                    "truncating_suffix" => Value::nothing(span),
                },
            },
            span,
        ),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TableIndent {
    pub left: usize,
    pub right: usize,
}

pub(super) fn reconstruct_padding(config: &Config, span: Span) -> Value {
    // For better completions always reconstruct the record version even though unsigned int would
    // be supported, `as` conversion is sane as it came from an i64 original
    Value::record(
        record!(
            "left" => Value::int(config.table_indent.left as i64, span),
            "right" => Value::int(config.table_indent.right as i64, span),
        ),
        span,
    )
}
