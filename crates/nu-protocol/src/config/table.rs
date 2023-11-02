use std::str::FromStr;

use crate::{record, Config, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

use super::helper::ReconstructVal;

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
    let map = value.as_record().map_err(|e| {
        ShellError::GenericError(
            "Error while applying config changes".into(),
            "$env.config.table.trim is not a record".into(),
            Some(value.span()),
            Some("Please consult the documentation for configuring Nushell.".into()),
            vec![e],
        )
    })?;

    let mut methodology = match map.get("methodology") {
        Some(value) => match try_parse_trim_methodology(value) {
            Some(methodology) => methodology,
            None => return Ok(TrimStrategy::default()),
        },
        None => {
            errors.push(ShellError::GenericError(
                "Error while applying config changes".into(),
                "$env.config.table.trim.methodology was not provided".into(),
                Some(value.span()),
                Some("Please consult the documentation for configuring Nushell.".into()),
                vec![],
            ));
            return Ok(TrimStrategy::default());
        }
    };

    match &mut methodology {
        TrimStrategy::Wrap { try_to_keep_words } => {
            if let Some(value) = map.get("wrapping_try_keep_words") {
                if let Ok(b) = value.as_bool() {
                    *try_to_keep_words = b;
                } else {
                    errors.push(ShellError::GenericError(
                        "Error while applying config changes".into(),
                        "$env.config.table.trim.wrapping_try_keep_words is not a bool".into(),
                        Some(value.span()),
                        Some("Please consult the documentation for configuring Nushell.".into()),
                        vec![],
                    ));
                }
            }
        }
        TrimStrategy::Truncate { suffix } => {
            if let Some(value) = map.get("truncating_suffix") {
                if let Ok(v) = value.as_string() {
                    *suffix = Some(v);
                } else {
                    errors.push(ShellError::GenericError(
                        "Error while applying config changes".into(),
                        "$env.config.table.trim.truncating_suffix is not a string".into(),
                        Some(value.span()),
                        Some("Please consult the documentation for configuring Nushell.".into()),
                        vec![],
                    ));
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
