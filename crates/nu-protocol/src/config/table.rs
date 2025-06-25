use super::{config_update_string_enum, prelude::*};
use crate as nu_protocol;

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
    Single,
    Double,
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
            "single" => Ok(Self::Single),
            "double" => Ok(Self::Double),
            _ => Err(
                "'basic', 'thin', 'light', 'compact', 'with_love', 'compact_double', 'rounded', 'reinforced', 'heavy', 'none', 'psql', 'markdown', 'dots', 'restructured', 'ascii_rounded', 'basic_compact', 'single', or 'double'",
            ),
        }
    }
}

impl UpdateFromValue for TableMode {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
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
    /// Calculate the screen height and row count, if screen height is larger than row count, don't show footer
    Auto,
}

impl FromStr for FooterMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "always" => Ok(FooterMode::Always),
            "never" => Ok(FooterMode::Never),
            "auto" => Ok(FooterMode::Auto),
            _ => Err("'never', 'always', 'auto', or int"),
        }
    }
}

impl UpdateFromValue for FooterMode {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        match value {
            Value::String { val, .. } => match val.parse() {
                Ok(val) => *self = val,
                Err(err) => errors.invalid_value(path, err.to_string(), value),
            },
            &Value::Int { val, .. } => {
                if val >= 0 {
                    *self = Self::RowCount(val as u64);
                } else {
                    errors.invalid_value(path, "a non-negative integer", value);
                }
            }
            _ => errors.type_mismatch(
                path,
                Type::custom("'never', 'always', 'auto', or int"),
                value,
            ),
        }
    }
}

impl IntoValue for FooterMode {
    fn into_value(self, span: Span) -> Value {
        match self {
            FooterMode::Always => "always".into_value(span),
            FooterMode::Never => "never".into_value(span),
            FooterMode::Auto => "auto".into_value(span),
            FooterMode::RowCount(c) => (c as i64).into_value(span),
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
            _ => Err("'never', 'always' or 'auto'"),
        }
    }
}

impl UpdateFromValue for TableIndexMode {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
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

impl UpdateFromValue for TrimStrategy {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        let Some(methodology) = record.get("methodology") else {
            errors.missing_column(path, "methodology", value.span());
            return;
        };

        match methodology.as_str() {
            Ok("wrapping") => {
                let mut try_to_keep_words = if let &mut Self::Wrap { try_to_keep_words } = self {
                    try_to_keep_words
                } else {
                    false
                };
                for (col, val) in record.iter() {
                    let path = &mut path.push(col);
                    match col.as_str() {
                        "wrapping_try_keep_words" => try_to_keep_words.update(val, path, errors),
                        "methodology" | "truncating_suffix" => (),
                        _ => errors.unknown_option(path, val),
                    }
                }
                *self = Self::Wrap { try_to_keep_words };
            }
            Ok("truncating") => {
                let mut suffix = if let Self::Truncate { suffix } = self {
                    suffix.take()
                } else {
                    None
                };
                for (col, val) in record.iter() {
                    let path = &mut path.push(col);
                    match col.as_str() {
                        "truncating_suffix" => match val {
                            Value::Nothing { .. } => suffix = None,
                            Value::String { val, .. } => suffix = Some(val.clone()),
                            _ => errors.type_mismatch(path, Type::String, val),
                        },
                        "methodology" | "wrapping_try_keep_words" => (),
                        _ => errors.unknown_option(path, val),
                    }
                }
                *self = Self::Truncate { suffix };
            }
            Ok(_) => errors.invalid_value(
                &path.push("methodology"),
                "'wrapping' or 'truncating'",
                methodology,
            ),
            Err(_) => errors.type_mismatch(&path.push("methodology"), Type::String, methodology),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableIndent {
    pub left: usize,
    pub right: usize,
}

impl TableIndent {
    pub fn new(left: usize, right: usize) -> Self {
        Self { left, right }
    }
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

impl UpdateFromValue for TableIndent {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        match value {
            &Value::Int { val, .. } => {
                if let Ok(val) = val.try_into() {
                    self.left = val;
                    self.right = val;
                } else {
                    errors.invalid_value(path, "a non-negative integer", value);
                }
            }
            Value::Record { val: record, .. } => {
                for (col, val) in record.iter() {
                    let path = &mut path.push(col);
                    match col.as_str() {
                        "left" => self.left.update(val, path, errors),
                        "right" => self.right.update(val, path, errors),
                        _ => errors.unknown_option(path, val),
                    }
                }
            }
            _ => errors.type_mismatch(path, Type::custom("int or record"), value),
        }
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
    pub footer_inheritance: bool,
    pub missing_value_symbol: String,
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
            "footer_inheritance" => self.footer_inheritance.into_value(span),
            "missing_value_symbol" => self.missing_value_symbol.into_value(span),
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
            footer_inheritance: false,
            missing_value_symbol: "❎".into(),
        }
    }
}

impl UpdateFromValue for TableConfig {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Value::Record { val: record, .. } = value else {
            errors.type_mismatch(path, Type::record(), value);
            return;
        };

        for (col, val) in record.iter() {
            let path = &mut path.push(col);
            match col.as_str() {
                "mode" => self.mode.update(val, path, errors),
                "index_mode" => self.index_mode.update(val, path, errors),
                "show_empty" => self.show_empty.update(val, path, errors),
                "trim" => self.trim.update(val, path, errors),
                "header_on_separator" => self.header_on_separator.update(val, path, errors),
                "padding" => self.padding.update(val, path, errors),
                "abbreviated_row_count" => match val {
                    Value::Nothing { .. } => self.abbreviated_row_count = None,
                    &Value::Int { val: count, .. } => {
                        if let Ok(count) = count.try_into() {
                            self.abbreviated_row_count = Some(count);
                        } else {
                            errors.invalid_value(path, "a non-negative integer", val);
                        }
                    }
                    _ => errors.type_mismatch(path, Type::custom("int or nothing"), val),
                },
                "footer_inheritance" => self.footer_inheritance.update(val, path, errors),
                "missing_value_symbol" => match val.as_str() {
                    Ok(val) => self.missing_value_symbol = val.to_string(),
                    Err(_) => errors.type_mismatch(path, Type::String, val),
                },
                _ => errors.unknown_option(path, val),
            }
        }
    }
}
