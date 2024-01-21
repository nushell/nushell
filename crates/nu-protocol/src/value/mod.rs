mod custom_value;
mod from;
mod from_value;
mod lazy_record;
mod path;
mod range;
mod record;
mod stream;
mod unit;

use crate::ast::{Bits, Boolean, CellPath, Comparison, PathMember};
use crate::ast::{Math, Operator};
use crate::engine::{Closure, EngineState};
use crate::ShellError;
use crate::{did_you_mean, BlockId, Config, Span, Spanned, Type};

use byte_unit::UnitType;
use chrono::{DateTime, Datelike, Duration, FixedOffset, Locale, TimeZone};
use chrono_humanize::HumanTime;
pub use custom_value::CustomValue;
use fancy_regex::Regex;
pub use from_value::FromValue;
pub use lazy_record::LazyRecord;
use nu_utils::{
    contains_emoji, get_system_locale, locale::get_system_locale_string, IgnoreCaseExt,
};
use num_format::ToFormattedString;
pub use path::*;
pub use range::*;
pub use record::Record;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::{
    borrow::Cow,
    fmt::{Display, Formatter, Result as FmtResult},
    path::PathBuf,
    {cmp::Ordering, fmt::Debug},
};
pub use stream::*;
pub use unit::*;

/// Core structured values that pass through the pipeline in Nushell.
// NOTE: Please do not reorder these enum cases without thinking through the
// impact on the PartialOrd implementation and the global sort order
#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Bool {
        val: bool,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Int {
        val: i64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Float {
        val: f64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Filesize {
        val: i64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Duration {
        val: i64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Date {
        val: DateTime<FixedOffset>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Range {
        val: Box<Range>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    String {
        val: String,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    QuotedString {
        val: String,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Record {
        val: Record,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    List {
        vals: Vec<Value>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Block {
        val: BlockId,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Closure {
        val: Closure,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Nothing {
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Error {
        error: Box<ShellError>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    Binary {
        val: Vec<u8>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    CellPath {
        val: CellPath,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    #[serde(skip_serializing)]
    CustomValue {
        val: Box<dyn CustomValue>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
    #[serde(skip)]
    LazyRecord {
        val: Box<dyn for<'a> LazyRecord<'a>>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        internal_span: Span,
    },
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Value::Bool { val, internal_span } => Value::bool(*val, *internal_span),
            Value::Int { val, internal_span } => Value::int(*val, *internal_span),
            Value::Filesize { val, internal_span } => Value::Filesize {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Duration { val, internal_span } => Value::Duration {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Date { val, internal_span } => Value::Date {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Range { val, internal_span } => Value::Range {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Float { val, internal_span } => Value::float(*val, *internal_span),
            Value::String { val, internal_span } => Value::String {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::QuotedString { val, internal_span } => Value::QuotedString {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Record { val, internal_span } => Value::Record {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::LazyRecord { val, internal_span } => val.clone_value(*internal_span),
            Value::List {
                vals,
                internal_span,
            } => Value::List {
                vals: vals.clone(),
                internal_span: *internal_span,
            },
            Value::Block { val, internal_span } => Value::Block {
                val: *val,
                internal_span: *internal_span,
            },
            Value::Closure { val, internal_span } => Value::Closure {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Nothing { internal_span } => Value::Nothing {
                internal_span: *internal_span,
            },
            Value::Error {
                error,
                internal_span,
            } => Value::Error {
                error: error.clone(),
                internal_span: *internal_span,
            },
            Value::Binary { val, internal_span } => Value::Binary {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::CellPath { val, internal_span } => Value::CellPath {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::CustomValue { val, internal_span } => val.clone_value(*internal_span),
        }
    }
}

impl Value {
    pub fn as_bool(&self) -> Result<bool, ShellError> {
        match self {
            Value::Bool { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "boolean".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_int(&self) -> Result<i64, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "int".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_float(&self) -> Result<f64, ShellError> {
        match self {
            Value::Float { val, .. } => Ok(*val),
            Value::Int { val, .. } => Ok(*val as f64),
            x => Err(ShellError::CantConvert {
                to_type: "float".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_filesize(&self) -> Result<i64, ShellError> {
        match self {
            Value::Filesize { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "filesize".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_duration(&self) -> Result<i64, ShellError> {
        match self {
            Value::Duration { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "duration".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_date(&self) -> Result<DateTime<FixedOffset>, ShellError> {
        match self {
            Value::Date { val, .. } => Ok(*val),
            x => Err(ShellError::CantConvert {
                to_type: "date".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_range(&self) -> Result<&Range, ShellError> {
        match self {
            Value::Range { val, .. } => Ok(val.as_ref()),
            x => Err(ShellError::CantConvert {
                to_type: "range".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    /// Converts into string values that can be changed into string natively
    pub fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(val.to_string()),
            Value::Float { val, .. } => Ok(val.to_string()),
            Value::String { val, .. } => Ok(val.to_string()),
            Value::Binary { val, .. } => Ok(match std::str::from_utf8(val) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    return Err(ShellError::CantConvert {
                        to_type: "string".into(),
                        from_type: "binary".into(),
                        span: self.span(),
                        help: None,
                    });
                }
            }),
            Value::Date { val, .. } => Ok(val.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
            x => Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_spanned_string(&self) -> Result<Spanned<String>, ShellError> {
        let span = self.span();
        match self {
            Value::String { val, .. } => Ok(Spanned {
                item: val.to_string(),
                span,
            }),
            Value::Binary { val, .. } => Ok(match std::str::from_utf8(val) {
                Ok(s) => Spanned {
                    item: s.to_string(),
                    span,
                },
                Err(_) => {
                    return Err(ShellError::CantConvert {
                        to_type: "string".into(),
                        from_type: "binary".into(),
                        span: self.span(),
                        help: None,
                    })
                }
            }),
            x => Err(ShellError::CantConvert {
                to_type: "string".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_char(&self) -> Result<char, ShellError> {
        let span = self.span();

        match self {
            Value::String { val, .. } => {
                let mut chars = val.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => Ok(c),
                    _ => Err(ShellError::MissingParameter {
                        param_name: "single character separator".into(),
                        span,
                    }),
                }
            }
            x => Err(ShellError::CantConvert {
                to_type: "char".into(),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }

    pub fn as_path(&self) -> Result<PathBuf, ShellError> {
        match self {
            Value::String { val, .. } => Ok(PathBuf::from(val)),
            x => Err(ShellError::CantConvert {
                to_type: "path".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_record(&self) -> Result<&Record, ShellError> {
        match self {
            Value::Record { val, .. } => Ok(val),
            x => Err(ShellError::CantConvert {
                to_type: "record".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_list(&self) -> Result<&[Value], ShellError> {
        match self {
            Value::List { vals, .. } => Ok(vals),
            x => Err(ShellError::CantConvert {
                to_type: "list".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_block(&self) -> Result<BlockId, ShellError> {
        match self {
            Value::Block { val, .. } => Ok(*val),
            Value::Closure { val, .. } => Ok(val.block_id),
            x => Err(ShellError::CantConvert {
                to_type: "block".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_closure(&self) -> Result<&Closure, ShellError> {
        match self {
            Value::Closure { val, .. } => Ok(val),
            x => Err(ShellError::CantConvert {
                to_type: "closure".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_binary(&self) -> Result<&[u8], ShellError> {
        match self {
            Value::Binary { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(val.as_bytes()),
            x => Err(ShellError::CantConvert {
                to_type: "binary".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_cell_path(&self) -> Result<&CellPath, ShellError> {
        match self {
            Value::CellPath { val, .. } => Ok(val),
            x => Err(ShellError::CantConvert {
                to_type: "cell path".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_custom_value(&self) -> Result<&dyn CustomValue, ShellError> {
        match self {
            Value::CustomValue { val, .. } => Ok(val.as_ref()),
            x => Err(ShellError::CantConvert {
                to_type: "custom value".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    pub fn as_lazy_record(&self) -> Result<&dyn for<'a> LazyRecord<'a>, ShellError> {
        match self {
            Value::LazyRecord { val, .. } => Ok(val.as_ref()),
            x => Err(ShellError::CantConvert {
                to_type: "lazy record".into(),
                from_type: x.get_type().to_string(),
                span: self.span(),
                help: None,
            }),
        }
    }

    /// Get the span for the current value
    pub fn span(&self) -> Span {
        match self {
            Value::Bool { internal_span, .. }
            | Value::Int { internal_span, .. }
            | Value::Float { internal_span, .. }
            | Value::Filesize { internal_span, .. }
            | Value::Duration { internal_span, .. }
            | Value::Date { internal_span, .. }
            | Value::Range { internal_span, .. }
            | Value::String { internal_span, .. }
            | Value::QuotedString { internal_span, .. }
            | Value::Record { internal_span, .. }
            | Value::List { internal_span, .. }
            | Value::Block { internal_span, .. }
            | Value::Closure { internal_span, .. }
            | Value::Nothing { internal_span, .. }
            | Value::Binary { internal_span, .. }
            | Value::CellPath { internal_span, .. }
            | Value::CustomValue { internal_span, .. }
            | Value::LazyRecord { internal_span, .. }
            | Value::Error { internal_span, .. } => *internal_span,
        }
    }

    /// Update the value with a new span
    pub fn with_span(mut self, new_span: Span) -> Value {
        match &mut self {
            Value::Bool { internal_span, .. }
            | Value::Int { internal_span, .. }
            | Value::Float { internal_span, .. }
            | Value::Filesize { internal_span, .. }
            | Value::Duration { internal_span, .. }
            | Value::Date { internal_span, .. }
            | Value::Range { internal_span, .. }
            | Value::String { internal_span, .. }
            | Value::QuotedString { internal_span, .. }
            | Value::Record { internal_span, .. }
            | Value::LazyRecord { internal_span, .. }
            | Value::List { internal_span, .. }
            | Value::Closure { internal_span, .. }
            | Value::Block { internal_span, .. }
            | Value::Nothing { internal_span, .. }
            | Value::Binary { internal_span, .. }
            | Value::CellPath { internal_span, .. }
            | Value::CustomValue { internal_span, .. } => *internal_span = new_span,
            Value::Error { .. } => (),
        }

        self
    }

    /// Get the type of the current Value
    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool { .. } => Type::Bool,
            Value::Int { .. } => Type::Int,
            Value::Float { .. } => Type::Float,
            Value::Filesize { .. } => Type::Filesize,
            Value::Duration { .. } => Type::Duration,
            Value::Date { .. } => Type::Date,
            Value::Range { .. } => Type::Range,
            Value::String { .. } => Type::String,
            Value::QuotedString { .. } => Type::String,
            Value::Record { val, .. } => {
                Type::Record(val.iter().map(|(x, y)| (x.clone(), y.get_type())).collect())
            }
            Value::List { vals, .. } => {
                let mut ty = None;
                for val in vals {
                    let val_ty = val.get_type();
                    match &ty {
                        Some(x) => {
                            if &val_ty != x {
                                if x.is_numeric() && val_ty.is_numeric() {
                                    ty = Some(Type::Number)
                                } else {
                                    ty = Some(Type::Any);
                                    break;
                                }
                            }
                        }
                        None => ty = Some(val_ty),
                    }
                }

                match ty {
                    Some(Type::Record(columns)) => Type::Table(columns),
                    Some(ty) => Type::List(Box::new(ty)),
                    None => Type::List(Box::new(Type::Any)),
                }
            }
            Value::LazyRecord { val, .. } => match val.collect() {
                Ok(val) => val.get_type(),
                Err(..) => Type::Error,
            },
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::Closure { .. } => Type::Closure,
            Value::Error { .. } => Type::Error,
            Value::Binary { .. } => Type::Binary,
            Value::CellPath { .. } => Type::CellPath,
            Value::CustomValue { val, .. } => Type::Custom(val.typetag_name().into()),
        }
    }

    pub fn get_data_by_key(&self, name: &str) -> Option<Value> {
        let span = self.span();
        match self {
            Value::Record { val, .. } => val.get(name).cloned(),
            Value::List { vals, .. } => {
                let out = vals
                    .iter()
                    .map(|item| {
                        item.as_record()
                            .ok()
                            .and_then(|val| val.get(name).cloned())
                            .unwrap_or(Value::nothing(span))
                    })
                    .collect::<Vec<_>>();

                if !out.is_empty() {
                    Some(Value::list(out, span))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // Convert Value into String, but propagate errors.
    pub fn nonerror_into_string(
        &self,
        separator: &str,
        config: &Config,
    ) -> Result<String, ShellError> {
        if let Value::Error { error, .. } = self {
            Err(*error.to_owned())
        } else {
            Ok(self.into_string(separator, config))
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_string(&self, separator: &str, config: &Config) -> String {
        let span = self.span();

        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize_from_conf(*val, config),
            Value::Duration { val, .. } => format_duration(*val),

            Value::Date { val, .. } => match &config.datetime_normal_format {
                Some(format) => self.format_datetime(val, format),
                None => {
                    format!(
                        "{} ({})",
                        if val.year() >= 0 {
                            val.to_rfc2822()
                        } else {
                            val.to_rfc3339()
                        },
                        HumanTime::from(*val),
                    )
                }
            },
            Value::Range { val, .. } => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String { val, .. } => val.clone(),
            Value::QuotedString { val, .. } => val.clone(),
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.into_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { val, .. } => format!(
                "{{{}}}",
                val.iter()
                    .map(|(x, y)| format!("{}: {}", x, y.into_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::LazyRecord { val, .. } => {
                let collected = match val.collect() {
                    Ok(val) => val,
                    Err(error) => Value::Error {
                        error: Box::new(error),
                        internal_span: span,
                    },
                };
                collected.into_string(separator, config)
            }
            Value::Block { val, .. } => format!("<Block {val}>"),
            Value::Closure { val, .. } => format!("<Closure {}>", val.block_id),
            Value::Nothing { .. } => String::new(),
            Value::Error { error, .. } => format!("{error:?}"),
            Value::Binary { val, .. } => format!("{val:?}"),
            Value::CellPath { val, .. } => val.to_string(),
            Value::CustomValue { val, .. } => val.value_string(),
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_abbreviated_string(&self, config: &Config) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize_from_conf(*val, config),
            Value::Duration { val, .. } => format_duration(*val),
            Value::Date { val, .. } => match &config.datetime_table_format {
                Some(format) => self.format_datetime(val, format),
                None => HumanTime::from(*val).to_string(),
            },
            Value::Range { val, .. } => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String { val, .. } => val.to_string(),
            Value::QuotedString { val, .. } => val.to_string(),
            Value::List { ref vals, .. } => {
                if !vals.is_empty() && vals.iter().all(|x| matches!(x, Value::Record { .. })) {
                    format!(
                        "[table {} row{}]",
                        vals.len(),
                        if vals.len() == 1 { "" } else { "s" }
                    )
                } else {
                    format!(
                        "[list {} item{}]",
                        vals.len(),
                        if vals.len() == 1 { "" } else { "s" }
                    )
                }
            }
            Value::Record { val, .. } => format!(
                "{{record {} field{}}}",
                val.len(),
                if val.len() == 1 { "" } else { "s" }
            ),
            Value::LazyRecord { val, .. } => match val.collect() {
                Ok(val) => val.into_abbreviated_string(config),
                Err(error) => format!("{error:?}"),
            },
            Value::Block { val, .. } => format!("<Block {val}>"),
            Value::Closure { val, .. } => format!("<Closure {}>", val.block_id),
            Value::Nothing { .. } => String::new(),
            Value::Error { error, .. } => format!("{error:?}"),
            Value::Binary { val, .. } => format!("{val:?}"),
            Value::CellPath { val, .. } => val.to_string(),
            Value::CustomValue { val, .. } => val.value_string(),
        }
    }

    fn format_datetime<Tz: TimeZone>(&self, date_time: &DateTime<Tz>, formatter: &str) -> String
    where
        Tz::Offset: Display,
    {
        let mut formatter_buf = String::new();
        // These are already in locale format, so we don't need to localize them
        let format = if ["%x", "%X", "%r"]
            .iter()
            .any(|item| formatter.contains(item))
        {
            date_time.format(formatter)
        } else {
            let locale: Locale = get_system_locale_string()
                .map(|l| l.replace('-', "_")) // `chrono::Locale` needs something like `xx_xx`, rather than `xx-xx`
                .unwrap_or_else(|| String::from("en_US"))
                .as_str()
                .try_into()
                .unwrap_or(Locale::en_US);
            date_time.format_localized(formatter, locale)
        };

        match formatter_buf.write_fmt(format_args!("{format}")) {
            Ok(_) => (),
            Err(_) => formatter_buf = format!("Invalid format string {}", formatter),
        }
        formatter_buf
    }

    /// Convert Value into a debug string
    pub fn debug_value(&self) -> String {
        match self {
            Value::String { val, .. } => {
                if contains_emoji(val) {
                    // This has to be an emoji, so let's display the code points that make it up.
                    format!(
                        "{:#?}",
                        Value::string(val.escape_unicode().to_string(), self.span())
                    )
                } else {
                    format!("{self:#?}")
                }
            }
            _ => format!("{self:#?}"),
        }
    }

    /// Convert Value into a parsable string (quote strings)
    /// bugbug other, rarer types not handled

    pub fn into_string_parsable(&self, separator: &str, config: &Config) -> String {
        match self {
            // give special treatment to the simple types to make them parsable
            Value::String { val, .. } => format!("'{}'", val),

            // recurse back into this function for recursive formatting
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.into_string_parsable(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { val, .. } => format!(
                "{{{}}}",
                val.iter()
                    .map(|(x, y)| format!("{}: {}", x, y.into_string_parsable(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),

            // defer to standard handling for types where standard representation is parsable
            _ => self.into_string(separator, config),
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn debug_string(&self, separator: &str, config: &Config) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize_from_conf(*val, config),
            Value::Duration { val, .. } => format_duration(*val),
            Value::Date { val, .. } => format!("{val:?}"),
            Value::Range { val, .. } => {
                format!(
                    "{}..{}",
                    val.from.into_string(", ", config),
                    val.to.into_string(", ", config)
                )
            }
            Value::String { val, .. } => val.clone(),
            Value::QuotedString { val, .. } => val.clone(),
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.into_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { val, .. } => format!(
                "{{{}}}",
                val.iter()
                    .map(|(x, y)| format!("{}: {}", x, y.into_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::LazyRecord { val, .. } => match val.collect() {
                Ok(val) => val.debug_string(separator, config),
                Err(error) => format!("{error:?}"),
            },
            Value::Block { val, .. } => format!("<Block {val}>"),
            Value::Closure { val, .. } => format!("<Closure {}>", val.block_id),
            Value::Nothing { .. } => String::new(),
            Value::Error { error, .. } => format!("{error:?}"),
            Value::Binary { val, .. } => format!("{val:?}"),
            Value::CellPath { val, .. } => val.to_string(),
            Value::CustomValue { val, .. } => val.value_string(),
        }
    }

    /// Follow a given cell path into the value: for example accessing select elements in a stream or list
    pub fn follow_cell_path(
        self,
        cell_path: &[PathMember],
        insensitive: bool,
    ) -> Result<Value, ShellError> {
        let mut current = self;

        for member in cell_path {
            match member {
                PathMember::Int {
                    val: count,
                    span: origin_span,
                    optional,
                } => {
                    // Treat a numeric path member as `select <val>`
                    match current {
                        Value::List { mut vals, .. } => {
                            if *count < vals.len() {
                                // `vals` is owned and will be dropped right after this,
                                // so we can `swap_remove` the value at index `count`
                                // without worrying about preserving order.
                                current = vals.swap_remove(*count);
                            } else if *optional {
                                return Ok(Value::nothing(*origin_span)); // short-circuit
                            } else if vals.is_empty() {
                                return Err(ShellError::AccessEmptyContent { span: *origin_span });
                            } else {
                                return Err(ShellError::AccessBeyondEnd {
                                    max_idx: vals.len() - 1,
                                    span: *origin_span,
                                });
                            }
                        }
                        Value::Binary { val, .. } => {
                            if let Some(item) = val.get(*count) {
                                current = Value::int(*item as i64, *origin_span);
                            } else if *optional {
                                return Ok(Value::nothing(*origin_span)); // short-circuit
                            } else if val.is_empty() {
                                return Err(ShellError::AccessEmptyContent { span: *origin_span });
                            } else {
                                return Err(ShellError::AccessBeyondEnd {
                                    max_idx: val.len() - 1,
                                    span: *origin_span,
                                });
                            }
                        }
                        Value::Range { val, .. } => {
                            if let Some(item) = val.into_range_iter(None)?.nth(*count) {
                                current = item;
                            } else if *optional {
                                return Ok(Value::nothing(*origin_span)); // short-circuit
                            } else {
                                return Err(ShellError::AccessBeyondEndOfStream {
                                    span: *origin_span,
                                });
                            }
                        }
                        Value::CustomValue { val, .. } => {
                            current = match val.follow_path_int(*count, *origin_span) {
                                Ok(val) => val,
                                Err(err) => {
                                    if *optional {
                                        return Ok(Value::nothing(*origin_span));
                                    // short-circuit
                                    } else {
                                        return Err(err);
                                    }
                                }
                            };
                        }
                        Value::Nothing { .. } if *optional => {
                            return Ok(Value::nothing(*origin_span)); // short-circuit
                        }
                        // Records (and tables) are the only built-in which support column names,
                        // so only use this message for them.
                        Value::Record { .. } => {
                            return Err(ShellError::TypeMismatch {
                                err_message:"Can't access record values with a row index. Try specifying a column name instead".into(),
                                span: *origin_span,
                            });
                        }
                        Value::Error { error, .. } => return Err(*error),
                        x => {
                            return Err(ShellError::IncompatiblePathAccess {
                                type_name: format!("{}", x.get_type()),
                                span: *origin_span,
                            });
                        }
                    }
                }
                PathMember::String {
                    val: column_name,
                    span: origin_span,
                    optional,
                } => {
                    let span = current.span();

                    match current {
                        Value::Record { val, .. } => {
                            // Make reverse iterate to avoid duplicate column leads to first value, actually last value is expected.
                            if let Some(found) = val.iter().rev().find(|x| {
                                if insensitive {
                                    x.0.eq_ignore_case(column_name)
                                } else {
                                    x.0 == column_name
                                }
                            }) {
                                current = found.1.clone(); // TODO: avoid clone here
                            } else if *optional {
                                return Ok(Value::nothing(*origin_span)); // short-circuit
                            } else if let Some(suggestion) =
                                did_you_mean(val.columns(), column_name)
                            {
                                return Err(ShellError::DidYouMean {
                                    suggestion,
                                    span: *origin_span,
                                });
                            } else {
                                return Err(ShellError::CantFindColumn {
                                    col_name: column_name.clone(),
                                    span: *origin_span,
                                    src_span: span,
                                });
                            }
                        }
                        Value::LazyRecord { val, .. } => {
                            let columns = val.column_names();

                            if let Some(col) = columns.iter().rev().find(|&col| {
                                if insensitive {
                                    col.eq_ignore_case(column_name)
                                } else {
                                    col == column_name
                                }
                            }) {
                                current = val.get_column_value(col)?;
                            } else if *optional {
                                return Ok(Value::nothing(*origin_span)); // short-circuit
                            } else if let Some(suggestion) = did_you_mean(&columns, column_name) {
                                return Err(ShellError::DidYouMean {
                                    suggestion,
                                    span: *origin_span,
                                });
                            } else {
                                return Err(ShellError::CantFindColumn {
                                    col_name: column_name.clone(),
                                    span: *origin_span,
                                    src_span: span,
                                });
                            }
                        }
                        // String access of Lists always means Table access.
                        // Create a List which contains each matching value for contained
                        // records in the source list.
                        Value::List { vals, .. } => {
                            let list = vals
                                .into_iter()
                                .map(|val| {
                                    let val_span = val.span();
                                    match val {
                                        Value::Record { val, .. } => {
                                            if let Some(found) = val.iter().rev().find(|x| {
                                                if insensitive {
                                                    x.0.eq_ignore_case(column_name)
                                                } else {
                                                    x.0 == column_name
                                                }
                                            }) {
                                                Ok(found.1.clone()) // TODO: avoid clone here
                                            } else if *optional {
                                                Ok(Value::nothing(*origin_span))
                                            } else if let Some(suggestion) =
                                                did_you_mean(val.columns(), column_name)
                                            {
                                                Err(ShellError::DidYouMean {
                                                    suggestion,
                                                    span: *origin_span,
                                                })
                                            } else {
                                                Err(ShellError::CantFindColumn {
                                                    col_name: column_name.clone(),
                                                    span: *origin_span,
                                                    src_span: val_span,
                                                })
                                            }
                                        }
                                        Value::Nothing { .. } if *optional => {
                                            Ok(Value::nothing(*origin_span))
                                        }
                                        _ => Err(ShellError::CantFindColumn {
                                            col_name: column_name.clone(),
                                            span: *origin_span,
                                            src_span: val_span,
                                        }),
                                    }
                                })
                                .collect::<Result<_, _>>()?;

                            current = Value::list(list, span);
                        }
                        Value::CustomValue { val, .. } => {
                            current = val.follow_path_string(column_name.clone(), *origin_span)?;
                        }
                        Value::Nothing { .. } if *optional => {
                            return Ok(Value::nothing(*origin_span)); // short-circuit
                        }
                        Value::Error { error, .. } => return Err(*error),
                        x => {
                            return Err(ShellError::IncompatiblePathAccess {
                                type_name: format!("{}", x.get_type()),
                                span: *origin_span,
                            });
                        }
                    }
                }
            }
        }
        // If a single Value::Error was produced by the above (which won't happen if nullify_errors is true), unwrap it now.
        // Note that Value::Errors inside Lists remain as they are, so that the rest of the list can still potentially be used.
        if let Value::Error { error, .. } = current {
            Err(*error)
        } else {
            Ok(current)
        }
    }

    /// Follow a given cell path into the value: for example accessing select elements in a stream or list
    pub fn upsert_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
    ) -> Result<(), ShellError> {
        let orig = self.clone();

        let new_val = callback(&orig.follow_cell_path(cell_path, false)?);

        match new_val {
            Value::Error { error, .. } => Err(*error),
            new_val => self.upsert_data_at_cell_path(cell_path, new_val),
        }
    }

    pub fn upsert_data_at_cell_path(
        &mut self,
        cell_path: &[PathMember],
        new_val: Value,
    ) -> Result<(), ShellError> {
        let v_span = self.span();
        if let Some((member, path)) = cell_path.split_first() {
            match member {
                PathMember::String {
                    val: col_name,
                    span,
                    ..
                } => match self {
                    Value::List { vals, .. } => {
                        for val in vals.iter_mut() {
                            match val {
                                Value::Record { val: record, .. } => {
                                    if let Some(val) = record.get_mut(col_name) {
                                        val.upsert_data_at_cell_path(path, new_val.clone())?;
                                    } else {
                                        let new_col = if path.is_empty() {
                                            new_val.clone()
                                        } else {
                                            let mut new_col =
                                                Value::record(Record::new(), new_val.span());
                                            new_col
                                                .upsert_data_at_cell_path(path, new_val.clone())?;
                                            new_col
                                        };
                                        record.push(col_name, new_col);
                                    }
                                }
                                Value::Error { error, .. } => return Err(*error.clone()),
                                v => {
                                    return Err(ShellError::CantFindColumn {
                                        col_name: col_name.clone(),
                                        span: *span,
                                        src_span: v.span(),
                                    });
                                }
                            }
                        }
                    }
                    Value::Record { val: record, .. } => {
                        if let Some(val) = record.get_mut(col_name) {
                            val.upsert_data_at_cell_path(path, new_val)?;
                        } else {
                            let new_col = if path.is_empty() {
                                new_val
                            } else {
                                let mut new_col = Value::record(Record::new(), new_val.span());
                                new_col.upsert_data_at_cell_path(path, new_val)?;
                                new_col
                            };
                            record.push(col_name, new_col);
                        }
                    }
                    Value::LazyRecord { val, .. } => {
                        // convert to Record first.
                        let mut record = val.collect()?;
                        record.upsert_data_at_cell_path(cell_path, new_val)?;
                        *self = record;
                    }
                    Value::Error { error, .. } => return Err(*error.clone()),
                    v => {
                        return Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: *span,
                            src_span: v.span(),
                        });
                    }
                },
                PathMember::Int {
                    val: row_num, span, ..
                } => match self {
                    Value::List { vals, .. } => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.upsert_data_at_cell_path(path, new_val)?;
                        } else if vals.len() != *row_num {
                            return Err(ShellError::InsertAfterNextFreeIndex {
                                available_idx: vals.len(),
                                span: *span,
                            });
                        } else if !path.is_empty() {
                            return Err(ShellError::AccessBeyondEnd {
                                max_idx: vals.len() - 1,
                                span: *span,
                            });
                        } else {
                            // If the upsert is at 1 + the end of the list, it's OK.
                            vals.push(new_val);
                        }
                    }
                    Value::Error { error, .. } => return Err(*error.clone()),
                    _ => {
                        return Err(ShellError::NotAList {
                            dst_span: *span,
                            src_span: v_span,
                        });
                    }
                },
            }
        } else {
            *self = new_val;
        }
        Ok(())
    }

    /// Follow a given cell path into the value: for example accessing select elements in a stream or list
    pub fn update_cell_path<'a>(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value + 'a>,
    ) -> Result<(), ShellError> {
        let orig = self.clone();

        let new_val = callback(&orig.follow_cell_path(cell_path, false)?);

        match new_val {
            Value::Error { error, .. } => Err(*error),
            new_val => self.update_data_at_cell_path(cell_path, new_val),
        }
    }

    pub fn update_data_at_cell_path(
        &mut self,
        cell_path: &[PathMember],
        new_val: Value,
    ) -> Result<(), ShellError> {
        let v_span = self.span();
        if let Some((member, path)) = cell_path.split_first() {
            match member {
                PathMember::String {
                    val: col_name,
                    span,
                    ..
                } => match self {
                    Value::List { vals, .. } => {
                        for val in vals.iter_mut() {
                            let v_span = val.span();
                            match val {
                                Value::Record { val: record, .. } => {
                                    if let Some(val) = record.get_mut(col_name) {
                                        val.update_data_at_cell_path(path, new_val.clone())?;
                                    } else {
                                        return Err(ShellError::CantFindColumn {
                                            col_name: col_name.clone(),
                                            span: *span,
                                            src_span: v_span,
                                        });
                                    }
                                }
                                Value::Error { error, .. } => return Err(*error.clone()),
                                v => {
                                    return Err(ShellError::CantFindColumn {
                                        col_name: col_name.clone(),
                                        span: *span,
                                        src_span: v.span(),
                                    });
                                }
                            }
                        }
                    }
                    Value::Record { val: record, .. } => {
                        if let Some(val) = record.get_mut(col_name) {
                            val.update_data_at_cell_path(path, new_val)?;
                        } else {
                            return Err(ShellError::CantFindColumn {
                                col_name: col_name.clone(),
                                span: *span,
                                src_span: v_span,
                            });
                        }
                    }
                    Value::LazyRecord { val, .. } => {
                        // convert to Record first.
                        let mut record = val.collect()?;
                        record.update_data_at_cell_path(cell_path, new_val)?;
                        *self = record;
                    }
                    Value::Error { error, .. } => return Err(*error.clone()),
                    v => {
                        return Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: *span,
                            src_span: v.span(),
                        });
                    }
                },
                PathMember::Int {
                    val: row_num, span, ..
                } => match self {
                    Value::List { vals, .. } => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            v.update_data_at_cell_path(path, new_val)?;
                        } else if vals.is_empty() {
                            return Err(ShellError::AccessEmptyContent { span: *span });
                        } else {
                            return Err(ShellError::AccessBeyondEnd {
                                max_idx: vals.len() - 1,
                                span: *span,
                            });
                        }
                    }
                    Value::Error { error, .. } => return Err(*error.clone()),
                    v => {
                        return Err(ShellError::NotAList {
                            dst_span: *span,
                            src_span: v.span(),
                        });
                    }
                },
            }
        } else {
            *self = new_val;
        }
        Ok(())
    }

    pub fn remove_data_at_cell_path(&mut self, cell_path: &[PathMember]) -> Result<(), ShellError> {
        match cell_path {
            [] => Ok(()),
            [member] => {
                let v_span = self.span();
                match member {
                    PathMember::String {
                        val: col_name,
                        span,
                        optional,
                    } => match self {
                        Value::List { vals, .. } => {
                            for val in vals.iter_mut() {
                                let v_span = val.span();
                                match val {
                                    Value::Record { val: record, .. } => {
                                        if record.remove(col_name).is_none() && !optional {
                                            return Err(ShellError::CantFindColumn {
                                                col_name: col_name.clone(),
                                                span: *span,
                                                src_span: v_span,
                                            });
                                        }
                                    }
                                    v => {
                                        return Err(ShellError::CantFindColumn {
                                            col_name: col_name.clone(),
                                            span: *span,
                                            src_span: v.span(),
                                        });
                                    }
                                }
                            }
                            Ok(())
                        }
                        Value::Record { val: record, .. } => {
                            if record.remove(col_name).is_none() && !optional {
                                return Err(ShellError::CantFindColumn {
                                    col_name: col_name.clone(),
                                    span: *span,
                                    src_span: v_span,
                                });
                            }
                            Ok(())
                        }
                        Value::LazyRecord { val, .. } => {
                            // convert to Record first.
                            let mut record = val.collect()?;
                            record.remove_data_at_cell_path(cell_path)?;
                            *self = record;
                            Ok(())
                        }
                        v => Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: *span,
                            src_span: v.span(),
                        }),
                    },
                    PathMember::Int {
                        val: row_num,
                        span,
                        optional,
                    } => match self {
                        Value::List { vals, .. } => {
                            if vals.get_mut(*row_num).is_some() {
                                vals.remove(*row_num);
                                Ok(())
                            } else if *optional {
                                Ok(())
                            } else if vals.is_empty() {
                                Err(ShellError::AccessEmptyContent { span: *span })
                            } else {
                                Err(ShellError::AccessBeyondEnd {
                                    max_idx: vals.len() - 1,
                                    span: *span,
                                })
                            }
                        }
                        v => Err(ShellError::NotAList {
                            dst_span: *span,
                            src_span: v.span(),
                        }),
                    },
                }
            }
            [member, path @ ..] => {
                let v_span = self.span();
                match member {
                    PathMember::String {
                        val: col_name,
                        span,
                        optional,
                    } => match self {
                        Value::List { vals, .. } => {
                            for val in vals.iter_mut() {
                                let v_span = val.span();
                                match val {
                                    Value::Record { val: record, .. } => {
                                        if let Some(val) = record.get_mut(col_name) {
                                            val.remove_data_at_cell_path(path)?;
                                        } else if !optional {
                                            return Err(ShellError::CantFindColumn {
                                                col_name: col_name.clone(),
                                                span: *span,
                                                src_span: v_span,
                                            });
                                        }
                                    }
                                    v => {
                                        return Err(ShellError::CantFindColumn {
                                            col_name: col_name.clone(),
                                            span: *span,
                                            src_span: v.span(),
                                        });
                                    }
                                }
                            }
                            Ok(())
                        }
                        Value::Record { val: record, .. } => {
                            if let Some(val) = record.get_mut(col_name) {
                                val.remove_data_at_cell_path(path)?;
                            } else if !optional {
                                return Err(ShellError::CantFindColumn {
                                    col_name: col_name.clone(),
                                    span: *span,
                                    src_span: v_span,
                                });
                            }
                            Ok(())
                        }
                        Value::LazyRecord { val, .. } => {
                            // convert to Record first.
                            let mut record = val.collect()?;
                            record.remove_data_at_cell_path(cell_path)?;
                            *self = record;
                            Ok(())
                        }
                        v => Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: *span,
                            src_span: v.span(),
                        }),
                    },
                    PathMember::Int {
                        val: row_num,
                        span,
                        optional,
                    } => match self {
                        Value::List { vals, .. } => {
                            if let Some(v) = vals.get_mut(*row_num) {
                                v.remove_data_at_cell_path(path)
                            } else if *optional {
                                Ok(())
                            } else if vals.is_empty() {
                                Err(ShellError::AccessEmptyContent { span: *span })
                            } else {
                                Err(ShellError::AccessBeyondEnd {
                                    max_idx: vals.len() - 1,
                                    span: *span,
                                })
                            }
                        }
                        v => Err(ShellError::NotAList {
                            dst_span: *span,
                            src_span: v.span(),
                        }),
                    },
                }
            }
        }
    }

    pub fn insert_data_at_cell_path(
        &mut self,
        cell_path: &[PathMember],
        new_val: Value,
        head_span: Span,
    ) -> Result<(), ShellError> {
        let v_span = self.span();
        if let Some((member, path)) = cell_path.split_first() {
            match member {
                PathMember::String {
                    val: col_name,
                    span,
                    ..
                } => match self {
                    Value::List { vals, .. } => {
                        for val in vals.iter_mut() {
                            let v_span = val.span();
                            match val {
                                Value::Record { val: record, .. } => {
                                    if let Some(val) = record.get_mut(col_name) {
                                        if path.is_empty() {
                                            return Err(ShellError::ColumnAlreadyExists {
                                                col_name: col_name.clone(),
                                                span: *span,
                                                src_span: v_span,
                                            });
                                        } else {
                                            val.insert_data_at_cell_path(
                                                path,
                                                new_val.clone(),
                                                head_span,
                                            )?;
                                        }
                                    } else {
                                        let new_col = if path.is_empty() {
                                            new_val.clone()
                                        } else {
                                            let mut new_col =
                                                Value::record(Record::new(), new_val.span());
                                            new_col.insert_data_at_cell_path(
                                                path,
                                                new_val.clone(),
                                                head_span,
                                            )?;
                                            new_col
                                        };
                                        record.push(col_name, new_col);
                                    }
                                }
                                Value::Error { error, .. } => return Err(*error.clone()),
                                _ => {
                                    return Err(ShellError::UnsupportedInput {
                                        msg: "expected table or record".into(),
                                        input: format!("input type: {:?}", val.get_type()),
                                        msg_span: head_span,
                                        input_span: *span,
                                    });
                                }
                            }
                        }
                    }
                    Value::Record { val: record, .. } => {
                        if let Some(val) = record.get_mut(col_name) {
                            if path.is_empty() {
                                return Err(ShellError::ColumnAlreadyExists {
                                    col_name: col_name.clone(),
                                    span: *span,
                                    src_span: v_span,
                                });
                            } else {
                                val.insert_data_at_cell_path(path, new_val, head_span)?;
                            }
                        } else {
                            let new_col = if path.is_empty() {
                                new_val.clone()
                            } else {
                                let mut new_col = Value::record(Record::new(), new_val.span());
                                new_col.insert_data_at_cell_path(
                                    path,
                                    new_val.clone(),
                                    head_span,
                                )?;
                                new_col
                            };
                            record.push(col_name, new_col);
                        }
                    }
                    Value::LazyRecord { val, .. } => {
                        // convert to Record first.
                        let mut record = val.collect()?;
                        record.insert_data_at_cell_path(cell_path, new_val, v_span)?;
                        *self = record;
                    }
                    other => {
                        return Err(ShellError::UnsupportedInput {
                            msg: "table or record".into(),
                            input: format!("input type: {:?}", other.get_type()),
                            msg_span: head_span,
                            input_span: *span,
                        });
                    }
                },
                PathMember::Int {
                    val: row_num, span, ..
                } => match self {
                    Value::List { vals, .. } => {
                        if let Some(v) = vals.get_mut(*row_num) {
                            if path.is_empty() {
                                vals.insert(*row_num, new_val);
                            } else {
                                v.insert_data_at_cell_path(path, new_val, head_span)?;
                            }
                        } else if vals.len() != *row_num {
                            return Err(ShellError::InsertAfterNextFreeIndex {
                                available_idx: vals.len(),
                                span: *span,
                            });
                        } else if !path.is_empty() {
                            return Err(ShellError::AccessBeyondEnd {
                                max_idx: vals.len() - 1,
                                span: *span,
                            });
                        } else {
                            // If the insert is at 1 + the end of the list, it's OK.
                            vals.push(new_val);
                        }
                    }
                    _ => {
                        return Err(ShellError::NotAList {
                            dst_span: *span,
                            src_span: v_span,
                        });
                    }
                },
            }
        } else {
            *self = new_val;
        }
        Ok(())
    }

    /// Check if the content is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Value::String { val, .. } => val.is_empty(),
            Value::List { vals, .. } => vals.is_empty(),
            Value::Record { val, .. } => val.is_empty(),
            Value::Binary { val, .. } => val.is_empty(),
            Value::Nothing { .. } => true,
            _ => false,
        }
    }

    pub fn is_nothing(&self) -> bool {
        matches!(self, Value::Nothing { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error { .. })
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Value::Bool { val: true, .. })
    }

    pub fn is_false(&self) -> bool {
        matches!(self, Value::Bool { val: false, .. })
    }

    pub fn columns(&self) -> impl Iterator<Item = &String> {
        let opt = match self {
            Value::Record { val, .. } => Some(val.columns()),
            _ => None,
        };

        opt.into_iter().flatten()
    }

    pub fn bool(val: bool, span: Span) -> Value {
        Value::Bool {
            val,
            internal_span: span,
        }
    }

    pub fn int(val: i64, span: Span) -> Value {
        Value::Int {
            val,
            internal_span: span,
        }
    }

    pub fn float(val: f64, span: Span) -> Value {
        Value::Float {
            val,
            internal_span: span,
        }
    }

    pub fn filesize(val: i64, span: Span) -> Value {
        Value::Filesize {
            val,
            internal_span: span,
        }
    }

    pub fn duration(val: i64, span: Span) -> Value {
        Value::Duration {
            val,
            internal_span: span,
        }
    }

    pub fn date(val: DateTime<FixedOffset>, span: Span) -> Value {
        Value::Date {
            val,
            internal_span: span,
        }
    }

    pub fn range(val: Range, span: Span) -> Value {
        Value::Range {
            val: Box::new(val),
            internal_span: span,
        }
    }

    pub fn string(val: impl Into<String>, span: Span) -> Value {
        Value::String {
            val: val.into(),
            internal_span: span,
        }
    }

    pub fn quoted_string(val: impl Into<String>, span: Span) -> Value {
        Value::QuotedString {
            val: val.into(),
            internal_span: span,
        }
    }

    pub fn record(val: Record, span: Span) -> Value {
        Value::Record {
            val,
            internal_span: span,
        }
    }

    pub fn list(vals: Vec<Value>, span: Span) -> Value {
        Value::List {
            vals,
            internal_span: span,
        }
    }

    pub fn block(val: BlockId, span: Span) -> Value {
        Value::Block {
            val,
            internal_span: span,
        }
    }

    pub fn closure(val: Closure, span: Span) -> Value {
        Value::Closure {
            val,
            internal_span: span,
        }
    }

    /// Create a new `Nothing` value
    pub fn nothing(span: Span) -> Value {
        Value::Nothing {
            internal_span: span,
        }
    }

    pub fn error(error: ShellError, span: Span) -> Value {
        Value::Error {
            error: Box::new(error),
            internal_span: span,
        }
    }

    pub fn binary(val: impl Into<Vec<u8>>, span: Span) -> Value {
        Value::Binary {
            val: val.into(),
            internal_span: span,
        }
    }

    pub fn cell_path(val: CellPath, span: Span) -> Value {
        Value::CellPath {
            val,
            internal_span: span,
        }
    }

    pub fn custom_value(val: Box<dyn CustomValue>, span: Span) -> Value {
        Value::CustomValue {
            val,
            internal_span: span,
        }
    }

    pub fn lazy_record(val: Box<dyn for<'a> LazyRecord<'a>>, span: Span) -> Value {
        Value::LazyRecord {
            val,
            internal_span: span,
        }
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_bool(val: bool) -> Value {
        Value::bool(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_int(val: i64) -> Value {
        Value::int(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_float(val: f64) -> Value {
        Value::float(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_filesize(val: i64) -> Value {
        Value::filesize(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_duration(val: i64) -> Value {
        Value::duration(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_date(val: DateTime<FixedOffset>) -> Value {
        Value::date(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_range(val: Range) -> Value {
        Value::range(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_string(val: impl Into<String>) -> Value {
        Value::string(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_record(val: Record) -> Value {
        Value::record(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_list(vals: Vec<Value>) -> Value {
        Value::list(vals, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_block(val: BlockId) -> Value {
        Value::block(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_closure(val: Closure) -> Value {
        Value::closure(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_nothing() -> Value {
        Value::nothing(Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_binary(val: impl Into<Vec<u8>>) -> Value {
        Value::binary(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_cell_path(val: CellPath) -> Value {
        Value::cell_path(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_custom_value(val: Box<dyn CustomValue>) -> Value {
        Value::custom_value(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_lazy_record(val: Box<dyn for<'a> LazyRecord<'a>>) -> Value {
        Value::lazy_record(val, Span::test_data())
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nothing {
            internal_span: Span::unknown(),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Compare two floating point numbers. The decision interval for equality is dynamically
        // scaled as the value being compared increases in magnitude.
        fn compare_floats(val: f64, other: f64) -> Option<Ordering> {
            let prec = f64::EPSILON.max(val.abs() * f64::EPSILON);

            if (other - val).abs() < prec {
                return Some(Ordering::Equal);
            }

            val.partial_cmp(&other)
        }

        match (self, other) {
            (Value::Bool { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Int { .. } => Some(Ordering::Less),
                Value::Float { .. } => Some(Ordering::Less),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::QuotedString { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Int { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Float { val: rhs, .. } => compare_floats(*lhs as f64, *rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::QuotedString { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Float { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { val: rhs, .. } => compare_floats(*lhs, *rhs as f64),
                Value::Float { val: rhs, .. } => compare_floats(*lhs, *rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::QuotedString { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Filesize { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::QuotedString { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Duration { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::QuotedString { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Date { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Range { .. } => Some(Ordering::Less),
                Value::String { .. } => Some(Ordering::Less),
                Value::QuotedString { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Range { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::String { .. } => Some(Ordering::Less),
                Value::QuotedString { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::String { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::QuotedString { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::QuotedString { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::QuotedString { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Record { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { val: rhs, .. } => {
                    // reorder cols and vals to make more logically compare.
                    // more general, if two record have same col and values,
                    // the order of cols shouldn't affect the equal property.
                    let (lhs_cols_ordered, lhs_vals_ordered) = reorder_record_inner(lhs);
                    let (rhs_cols_ordered, rhs_vals_ordered) = reorder_record_inner(rhs);

                    let result = lhs_cols_ordered.partial_cmp(&rhs_cols_ordered);
                    if result == Some(Ordering::Equal) {
                        lhs_vals_ordered.partial_cmp(&rhs_vals_ordered)
                    } else {
                        result
                    }
                }
                Value::LazyRecord { val, .. } => {
                    if let Ok(rhs) = val.collect() {
                        self.partial_cmp(&rhs)
                    } else {
                        None
                    }
                }
                Value::List { .. } => Some(Ordering::Less),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::List { vals: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { vals: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Block { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Block { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::Block { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Closure { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { val: rhs, .. } => lhs.block_id.partial_cmp(&rhs.block_id),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Nothing { .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Equal),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Error { .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Equal),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::Binary { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::CellPath { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::QuotedString { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Block { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { .. } => Some(Ordering::Greater),
                Value::CellPath { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::CustomValue { .. } => Some(Ordering::Less),
            },
            (Value::CustomValue { val: lhs, .. }, rhs) => lhs.partial_cmp(rhs),
            (Value::LazyRecord { val, .. }, rhs) => {
                if let Ok(val) = val.collect() {
                    val.partial_cmp(rhs)
                } else {
                    None
                }
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).map_or(false, Ordering::is_eq)
    }
}

impl Value {
    pub fn add(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow { msg: "add operation overflowed".into(), span, help: "Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into() })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float(*lhs as f64 + *rhs, span))
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::float(*lhs + *rhs as f64, span))
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float(lhs + rhs, span))
            }
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                Ok(Value::string(lhs.to_string() + rhs, span))
            }

            (Value::Date { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add_signed(chrono::Duration::nanoseconds(*rhs)) {
                    Ok(Value::date(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "addition operation overflowed".into(),
                        span,
                        help: "".into(),
                    })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::duration(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "add operation overflowed".into(),
                        span,
                        help: "".into(),
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "add operation overflowed".into(),
                        span,
                        help: "".into(),
                    })
                }
            }

            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Plus), op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn append(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::List { vals: lhs, .. }, Value::List { vals: rhs, .. }) => {
                let mut lhs = lhs.clone();
                let mut rhs = rhs.clone();
                lhs.append(&mut rhs);
                Ok(Value::list(lhs, span))
            }
            (Value::List { vals: lhs, .. }, val) => {
                let mut lhs = lhs.clone();
                lhs.push(val.clone());
                Ok(Value::list(lhs, span))
            }
            (val, Value::List { vals: rhs, .. }) => {
                let mut rhs = rhs.clone();
                rhs.insert(0, val.clone());
                Ok(Value::list(rhs, span))
            }
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                Ok(Value::string(lhs.to_string() + rhs, span))
            }
            (Value::Binary { val: lhs, .. }, Value::Binary { val: rhs, .. }) => {
                let mut val = lhs.clone();
                val.extend(rhs);
                Ok(Value::binary(val, span))
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn sub(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_sub(*rhs) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow { msg: "subtraction operation overflowed".into(), span, help: "Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into() })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float(*lhs as f64 - *rhs, span))
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::float(*lhs - *rhs as f64, span))
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float(lhs - rhs, span))
            }
            (Value::Date { val: lhs, .. }, Value::Date { val: rhs, .. }) => {
                let result = lhs.signed_duration_since(*rhs);

                if let Some(v) = result.num_nanoseconds() {
                    Ok(Value::duration(v, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "subtraction operation overflowed".into(),
                        span,
                        help: "".into(),
                    })
                }
            }
            (Value::Date { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                match lhs.checked_sub_signed(chrono::Duration::nanoseconds(*rhs)) {
                    Some(val) => Ok(Value::date(val, span)),
                    _ => Err(ShellError::OperatorOverflow {
                        msg: "subtraction operation overflowed".into(),
                        span,
                        help: "".into(),
                    }),
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_sub(*rhs) {
                    Ok(Value::duration(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "subtraction operation overflowed".into(),
                        span,
                        help: "".into(),
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_sub(*rhs) {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "add operation overflowed".into(),
                        span,
                        help: "".into(),
                    })
                }
            }

            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Minus), op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn mul(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_mul(*rhs) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow { msg: "multiply operation overflowed".into(), span, help: "Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into() })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float(*lhs as f64 * *rhs, span))
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::float(*lhs * *rhs as f64, span))
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float(lhs * rhs, span))
            }
            (Value::Int { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::filesize(*lhs * *rhs, span))
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::filesize(*lhs * *rhs, span))
            }
            (Value::Float { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::filesize((*lhs * *rhs as f64) as i64, span))
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::filesize((*lhs as f64 * *rhs) as i64, span))
            }
            (Value::Int { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::duration(*lhs * *rhs, span))
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::duration(*lhs * *rhs, span))
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::duration((*lhs as f64 * *rhs) as i64, span))
            }
            (Value::Float { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::duration((*lhs * *rhs as f64) as i64, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Multiply), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn div(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    if lhs % rhs == 0 {
                        Ok(Value::int(lhs / rhs, span))
                    } else {
                        Ok(Value::float((*lhs as f64) / (*rhs as f64), span))
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::float(*lhs as f64 / *rhs, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::float(*lhs / *rhs as f64, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::float(lhs / rhs, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if *rhs != 0 {
                    if lhs % rhs == 0 {
                        Ok(Value::int(lhs / rhs, span))
                    } else {
                        Ok(Value::float((*lhs as f64) / (*rhs as f64), span))
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::filesize(
                        ((*lhs as f64) / (*rhs as f64)) as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::filesize((*lhs as f64 / rhs) as i64, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if *rhs != 0 {
                    if lhs % rhs == 0 {
                        Ok(Value::int(lhs / rhs, span))
                    } else {
                        Ok(Value::float((*lhs as f64) / (*rhs as f64), span))
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::duration(
                        ((*lhs as f64) / (*rhs as f64)) as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::duration(((*lhs as f64) / rhs) as i64, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Divide), op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn floor_div(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::int(
                        (*lhs as f64 / *rhs as f64)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::int(
                        (*lhs as f64 / *rhs)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::int(
                        (*lhs / *rhs as f64)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::int(
                        (lhs / rhs)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::int(
                        (*lhs as f64 / *rhs as f64)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::filesize(
                        ((*lhs as f64) / (*rhs as f64))
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::filesize(
                        (*lhs as f64 / *rhs)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::int(
                        (*lhs as f64 / *rhs as f64)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::duration(
                        (*lhs as f64 / *rhs as f64)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::duration(
                        (*lhs as f64 / *rhs)
                            .clamp(std::i64::MIN as f64, std::i64::MAX as f64)
                            .floor() as i64,
                        span,
                    ))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Divide), op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn lt(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, .. }, rhs) = (self, rhs) {
            return lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::LessThan),
                op,
                rhs,
            );
        }

        if matches!(self, Value::Nothing { .. }) || matches!(rhs, Value::Nothing { .. }) {
            return Ok(Value::nothing(span));
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            });
        }

        if let Some(ordering) = self.partial_cmp(rhs) {
            Ok(Value::bool(matches!(ordering, Ordering::Less), span))
        } else {
            Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            })
        }
    }

    pub fn lte(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, .. }, rhs) = (self, rhs) {
            return lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::LessThanOrEqual),
                op,
                rhs,
            );
        }

        if matches!(self, Value::Nothing { .. }) || matches!(rhs, Value::Nothing { .. }) {
            return Ok(Value::nothing(span));
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            });
        }

        self.partial_cmp(rhs)
            .map(|ordering| Value::bool(matches!(ordering, Ordering::Less | Ordering::Equal), span))
            .ok_or(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            })
    }

    pub fn gt(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, .. }, rhs) = (self, rhs) {
            return lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::GreaterThan),
                op,
                rhs,
            );
        }

        if matches!(self, Value::Nothing { .. }) || matches!(rhs, Value::Nothing { .. }) {
            return Ok(Value::nothing(span));
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            });
        }

        self.partial_cmp(rhs)
            .map(|ordering| Value::bool(matches!(ordering, Ordering::Greater), span))
            .ok_or(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            })
    }

    pub fn gte(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, .. }, rhs) = (self, rhs) {
            return lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::GreaterThanOrEqual),
                op,
                rhs,
            );
        }

        if matches!(self, Value::Nothing { .. }) || matches!(rhs, Value::Nothing { .. }) {
            return Ok(Value::nothing(span));
        }

        if !type_compatible(self.get_type(), rhs.get_type())
            && (self.get_type() != Type::Any)
            && (rhs.get_type() != Type::Any)
        {
            return Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            });
        }

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::bool(
                matches!(ordering, Ordering::Greater | Ordering::Equal),
                span,
            )),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn eq(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, .. }, rhs) = (self, rhs) {
            return lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::Equal),
                op,
                rhs,
            );
        }

        if let Some(ordering) = self.partial_cmp(rhs) {
            Ok(Value::bool(matches!(ordering, Ordering::Equal), span))
        } else {
            match (self, rhs) {
                (Value::Nothing { .. }, _) | (_, Value::Nothing { .. }) => {
                    Ok(Value::bool(false, span))
                }
                _ => Err(ShellError::OperatorMismatch {
                    op_span: op,
                    lhs_ty: self.get_type().to_string(),
                    lhs_span: self.span(),
                    rhs_ty: rhs.get_type().to_string(),
                    rhs_span: rhs.span(),
                }),
            }
        }
    }

    pub fn ne(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        if let (Value::CustomValue { val: lhs, .. }, rhs) = (self, rhs) {
            return lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::NotEqual),
                op,
                rhs,
            );
        }

        if let Some(ordering) = self.partial_cmp(rhs) {
            Ok(Value::bool(!matches!(ordering, Ordering::Equal), span))
        } else {
            match (self, rhs) {
                (Value::Nothing { .. }, _) | (_, Value::Nothing { .. }) => {
                    Ok(Value::bool(true, span))
                }
                _ => Err(ShellError::OperatorMismatch {
                    op_span: op,
                    lhs_ty: self.get_type().to_string(),
                    lhs_span: self.span(),
                    rhs_ty: rhs.get_type().to_string(),
                    rhs_span: rhs.span(),
                }),
            }
        }
    }

    pub fn r#in(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (lhs, Value::Range { val: rhs, .. }) => Ok(Value::bool(rhs.contains(lhs), span)),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                Ok(Value::bool(rhs.contains(lhs), span))
            }
            (lhs, Value::List { vals: rhs, .. }) => Ok(Value::bool(rhs.contains(lhs), span)),
            (Value::String { val: lhs, .. }, Value::Record { val: rhs, .. }) => {
                Ok(Value::bool(rhs.contains(lhs), span))
            }
            (Value::String { .. } | Value::Int { .. }, Value::CellPath { val: rhs, .. }) => {
                let val = rhs.members.iter().any(|member| match (self, member) {
                    (Value::Int { val: lhs, .. }, PathMember::Int { val: rhs, .. }) => {
                        *lhs == *rhs as i64
                    }
                    (Value::String { val: lhs, .. }, PathMember::String { val: rhs, .. }) => {
                        lhs == rhs
                    }
                    (Value::String { .. }, PathMember::Int { .. })
                    | (Value::Int { .. }, PathMember::String { .. }) => false,
                    _ => unreachable!(
                        "outer match arm ensures `self` is either a `String` or `Int` variant"
                    ),
                });

                Ok(Value::bool(val, span))
            }
            (Value::CellPath { val: lhs, .. }, Value::CellPath { val: rhs, .. }) => {
                Ok(Value::bool(
                    rhs.members
                        .windows(lhs.members.len())
                        .any(|member_window| member_window == rhs.members),
                    span,
                ))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Comparison(Comparison::In), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn not_in(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (lhs, Value::Range { val: rhs, .. }) => Ok(Value::bool(!rhs.contains(lhs), span)),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                Ok(Value::bool(!rhs.contains(lhs), span))
            }
            (lhs, Value::List { vals: rhs, .. }) => Ok(Value::bool(!rhs.contains(lhs), span)),
            (Value::String { val: lhs, .. }, Value::Record { val: rhs, .. }) => {
                Ok(Value::bool(!rhs.contains(lhs), span))
            }
            (Value::String { .. } | Value::Int { .. }, Value::CellPath { val: rhs, .. }) => {
                let val = rhs.members.iter().any(|member| match (self, member) {
                    (Value::Int { val: lhs, .. }, PathMember::Int { val: rhs, .. }) => {
                        *lhs != *rhs as i64
                    }
                    (Value::String { val: lhs, .. }, PathMember::String { val: rhs, .. }) => {
                        lhs != rhs
                    }
                    (Value::String { .. }, PathMember::Int { .. })
                    | (Value::Int { .. }, PathMember::String { .. }) => true,
                    _ => unreachable!(
                        "outer match arm ensures `self` is either a `String` or `Int` variant"
                    ),
                });

                Ok(Value::bool(val, span))
            }
            (Value::CellPath { val: lhs, .. }, Value::CellPath { val: rhs, .. }) => {
                Ok(Value::bool(
                    rhs.members
                        .windows(lhs.members.len())
                        .all(|member_window| member_window != rhs.members),
                    span,
                ))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::NotIn),
                op,
                rhs,
            ),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn regex_match(
        &self,
        engine_state: &EngineState,
        op: Span,
        rhs: &Value,
        invert: bool,
        span: Span,
    ) -> Result<Value, ShellError> {
        let rhs_span = rhs.span();
        match (self, rhs) {
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                let is_match = match engine_state.regex_cache.try_lock() {
                    Ok(mut cache) => {
                        if let Some(regex) = cache.get(rhs) {
                            regex.is_match(lhs)
                        } else {
                            let regex =
                                Regex::new(rhs).map_err(|e| ShellError::UnsupportedInput {
                                    msg: format!("{e}"),
                                    input: "value originated from here".into(),
                                    msg_span: span,
                                    input_span: rhs_span,
                                })?;
                            let ret = regex.is_match(lhs);
                            cache.put(rhs.clone(), regex);
                            ret
                        }
                    }
                    Err(_) => {
                        let regex = Regex::new(rhs).map_err(|e| ShellError::UnsupportedInput {
                            msg: format!("{e}"),
                            input: "value originated from here".into(),
                            msg_span: span,
                            input_span: rhs_span,
                        })?;
                        regex.is_match(lhs)
                    }
                };

                Ok(Value::bool(
                    if invert {
                        !is_match.unwrap_or(false)
                    } else {
                        is_match.unwrap_or(true)
                    },
                    span,
                ))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => lhs.operation(
                span,
                if invert {
                    Operator::Comparison(Comparison::NotRegexMatch)
                } else {
                    Operator::Comparison(Comparison::RegexMatch)
                },
                op,
                rhs,
            ),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn starts_with(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                Ok(Value::bool(lhs.starts_with(rhs), span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::StartsWith),
                op,
                rhs,
            ),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn ends_with(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                Ok(Value::bool(lhs.ends_with(rhs), span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => lhs.operation(
                self.span(),
                Operator::Comparison(Comparison::EndsWith),
                op,
                rhs,
            ),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn bit_shl(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::int(*lhs << rhs, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Bits(Bits::ShiftLeft), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn bit_shr(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::int(*lhs >> rhs, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Bits(Bits::ShiftRight), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn bit_or(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::int(*lhs | rhs, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Bits(Bits::BitOr), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn bit_xor(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::int(*lhs ^ rhs, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Bits(Bits::BitXor), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn bit_and(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::int(*lhs & rhs, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Bits(Bits::BitAnd), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn modulo(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::int(lhs % rhs, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::float(*lhs as f64 % *rhs, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::float(*lhs % *rhs as f64, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::float(lhs % rhs, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::duration(lhs % rhs, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Math(Math::Modulo), op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn and(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => {
                Ok(Value::bool(*lhs && *rhs, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Boolean(Boolean::And), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn or(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => {
                Ok(Value::bool(*lhs || *rhs, span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Boolean(Boolean::Or), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn xor(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => {
                Ok(Value::bool((*lhs && !*rhs) || (!*lhs && *rhs), span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Boolean(Boolean::Xor), op, rhs)
            }
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }

    pub fn pow(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_pow(*rhs as u32) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow { msg: "pow operation overflowed".into(), span, help: "Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into() })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float((*lhs as f64).powf(*rhs), span))
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                Ok(Value::float(lhs.powf(*rhs as f64), span))
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                Ok(Value::float(lhs.powf(*rhs), span))
            }
            (Value::CustomValue { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Math(Math::Pow), op, rhs)
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type().to_string(),
                lhs_span: self.span(),
                rhs_ty: rhs.get_type().to_string(),
                rhs_span: rhs.span(),
            }),
        }
    }
}

fn reorder_record_inner(record: &Record) -> (Vec<&String>, Vec<&Value>) {
    let mut kv_pairs = record.iter().collect::<Vec<_>>();
    kv_pairs.sort_by_key(|(col, _)| *col);
    kv_pairs.into_iter().unzip()
}

fn type_compatible(a: Type, b: Type) -> bool {
    if a == b {
        return true;
    }

    matches!((a, b), (Type::Int, Type::Float) | (Type::Float, Type::Int))
}

/// Is the given year a leap year?
#[allow(clippy::nonminimal_bool)]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0) && (year % 100 != 0 || (year % 100 == 0 && year % 400 == 0))
}

#[derive(Clone, Copy)]
pub enum TimePeriod {
    Nanos(i64),
    Micros(i64),
    Millis(i64),
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
    Years(i64),
}

impl TimePeriod {
    pub fn to_text(self) -> Cow<'static, str> {
        match self {
            Self::Nanos(n) => format!("{n} ns").into(),
            Self::Micros(n) => format!("{n} µs").into(),
            Self::Millis(n) => format!("{n} ms").into(),
            Self::Seconds(n) => format!("{n} sec").into(),
            Self::Minutes(n) => format!("{n} min").into(),
            Self::Hours(n) => format!("{n} hr").into(),
            Self::Days(n) => format!("{n} day").into(),
            Self::Weeks(n) => format!("{n} wk").into(),
            Self::Months(n) => format!("{n} month").into(),
            Self::Years(n) => format!("{n} yr").into(),
        }
    }
}

impl Display for TimePeriod {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_text())
    }
}

pub fn format_duration(duration: i64) -> String {
    let (sign, periods) = format_duration_as_timeperiod(duration);

    let text = periods
        .into_iter()
        .map(|p| p.to_text().to_string().replace(' ', ""))
        .collect::<Vec<String>>();

    format!(
        "{}{}",
        if sign == -1 { "-" } else { "" },
        text.join(" ").trim()
    )
}

pub fn format_duration_as_timeperiod(duration: i64) -> (i32, Vec<TimePeriod>) {
    // Attribution: most of this is taken from chrono-humanize-rs. Thanks!
    // https://gitlab.com/imp/chrono-humanize-rs/-/blob/master/src/humantime.rs
    // Current duration doesn't know a date it's based on, weeks is the max time unit it can normalize into.
    // Don't guess or estimate how many years or months it might contain.

    let (sign, duration) = if duration >= 0 {
        (1, duration)
    } else {
        (-1, -duration)
    };

    let dur = Duration::nanoseconds(duration);

    /// Split this a duration into number of whole weeks and the remainder
    fn split_weeks(duration: Duration) -> (Option<i64>, Duration) {
        let weeks = duration.num_weeks();
        let remainder = duration - Duration::weeks(weeks);
        normalize_split(weeks, remainder)
    }

    /// Split this a duration into number of whole days and the remainder
    fn split_days(duration: Duration) -> (Option<i64>, Duration) {
        let days = duration.num_days();
        let remainder = duration - Duration::days(days);
        normalize_split(days, remainder)
    }

    /// Split this a duration into number of whole hours and the remainder
    fn split_hours(duration: Duration) -> (Option<i64>, Duration) {
        let hours = duration.num_hours();
        let remainder = duration - Duration::hours(hours);
        normalize_split(hours, remainder)
    }

    /// Split this a duration into number of whole minutes and the remainder
    fn split_minutes(duration: Duration) -> (Option<i64>, Duration) {
        let minutes = duration.num_minutes();
        let remainder = duration - Duration::minutes(minutes);
        normalize_split(minutes, remainder)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_seconds(duration: Duration) -> (Option<i64>, Duration) {
        let seconds = duration.num_seconds();
        let remainder = duration - Duration::seconds(seconds);
        normalize_split(seconds, remainder)
    }

    /// Split this a duration into number of whole milliseconds and the remainder
    fn split_milliseconds(duration: Duration) -> (Option<i64>, Duration) {
        let millis = duration.num_milliseconds();
        let remainder = duration - Duration::milliseconds(millis);
        normalize_split(millis, remainder)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_microseconds(duration: Duration) -> (Option<i64>, Duration) {
        let micros = duration.num_microseconds().unwrap_or_default();
        let remainder = duration - Duration::microseconds(micros);
        normalize_split(micros, remainder)
    }

    /// Split this a duration into number of whole seconds and the remainder
    fn split_nanoseconds(duration: Duration) -> (Option<i64>, Duration) {
        let nanos = duration.num_nanoseconds().unwrap_or_default();
        let remainder = duration - Duration::nanoseconds(nanos);
        normalize_split(nanos, remainder)
    }

    fn normalize_split(
        wholes: impl Into<Option<i64>>,
        remainder: Duration,
    ) -> (Option<i64>, Duration) {
        let wholes = wholes.into().map(i64::abs).filter(|x| *x > 0);
        (wholes, remainder)
    }

    let mut periods = vec![];

    let (weeks, remainder) = split_weeks(dur);
    if let Some(weeks) = weeks {
        periods.push(TimePeriod::Weeks(weeks));
    }

    let (days, remainder) = split_days(remainder);
    if let Some(days) = days {
        periods.push(TimePeriod::Days(days));
    }

    let (hours, remainder) = split_hours(remainder);
    if let Some(hours) = hours {
        periods.push(TimePeriod::Hours(hours));
    }

    let (minutes, remainder) = split_minutes(remainder);
    if let Some(minutes) = minutes {
        periods.push(TimePeriod::Minutes(minutes));
    }

    let (seconds, remainder) = split_seconds(remainder);
    if let Some(seconds) = seconds {
        periods.push(TimePeriod::Seconds(seconds));
    }

    let (millis, remainder) = split_milliseconds(remainder);
    if let Some(millis) = millis {
        periods.push(TimePeriod::Millis(millis));
    }

    let (micros, remainder) = split_microseconds(remainder);
    if let Some(micros) = micros {
        periods.push(TimePeriod::Micros(micros));
    }

    let (nanos, _remainder) = split_nanoseconds(remainder);
    if let Some(nanos) = nanos {
        periods.push(TimePeriod::Nanos(nanos));
    }

    if periods.is_empty() {
        periods.push(TimePeriod::Seconds(0));
    }

    (sign, periods)
}

pub fn format_filesize_from_conf(num_bytes: i64, config: &Config) -> String {
    // We need to take into account config.filesize_metric so, if someone asks for KB
    // and filesize_metric is false, return KiB
    format_filesize(
        num_bytes,
        config.filesize_format.as_str(),
        Some(config.filesize_metric),
    )
}

// filesize_metric is explicit when printed a value according to user config;
// other places (such as `format filesize`) don't.
pub fn format_filesize(
    num_bytes: i64,
    format_value: &str,
    filesize_metric: Option<bool>,
) -> String {
    // Allow the user to specify how they want their numbers formatted

    // When format_value is "auto" or an invalid value, the returned ByteUnit doesn't matter
    // and is always B.
    let filesize_unit = get_filesize_format(format_value, filesize_metric);
    let byte = byte_unit::Byte::from_u64(num_bytes.unsigned_abs());
    let adj_byte = if let Some(unit) = filesize_unit {
        byte.get_adjusted_unit(unit)
    } else {
        // When filesize_metric is None, format_value should never be "auto", so this
        // unwrap_or() should always work.
        byte.get_appropriate_unit(if filesize_metric.unwrap_or(false) {
            UnitType::Decimal
        } else {
            UnitType::Binary
        })
    };

    match adj_byte.get_unit() {
        byte_unit::Unit::B => {
            let locale = get_system_locale();
            let locale_byte = adj_byte.get_value() as u64;
            let locale_byte_string = locale_byte.to_formatted_string(&locale);
            let locale_signed_byte_string = if num_bytes.is_negative() {
                format!("-{locale_byte_string}")
            } else {
                locale_byte_string
            };

            if filesize_unit.is_none() {
                format!("{locale_signed_byte_string} B")
            } else {
                locale_signed_byte_string
            }
        }
        _ => {
            if num_bytes.is_negative() {
                format!("-{:.1}", adj_byte)
            } else {
                format!("{:.1}", adj_byte)
            }
        }
    }
}

/// Get the filesize unit, or None if format is "auto"
fn get_filesize_format(
    format_value: &str,
    filesize_metric: Option<bool>,
) -> Option<byte_unit::Unit> {
    // filesize_metric always overrides the unit of filesize_format.
    let metric = filesize_metric.unwrap_or(!format_value.ends_with("ib"));
    macro_rules! either {
        ($metric:ident, $binary:ident) => {
            Some(if metric {
                byte_unit::Unit::$metric
            } else {
                byte_unit::Unit::$binary
            })
        };
    }
    match format_value {
        "b" => Some(byte_unit::Unit::B),
        "kb" | "kib" => either!(KB, KiB),
        "mb" | "mib" => either!(MB, MiB),
        "gb" | "gib" => either!(GB, GiB),
        "tb" | "tib" => either!(TB, TiB),
        "pb" | "pib" => either!(TB, TiB),
        "eb" | "eib" => either!(EB, EiB),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Record, Value};
    use crate::record;

    mod is_empty {
        use super::*;

        #[test]
        fn test_string() {
            let value = Value::test_string("");
            assert!(value.is_empty());
        }

        #[test]
        fn test_list() {
            let list_with_no_values = Value::test_list(vec![]);
            let list_with_one_empty_string = Value::test_list(vec![Value::test_string("")]);

            assert!(list_with_no_values.is_empty());
            assert!(!list_with_one_empty_string.is_empty());
        }

        #[test]
        fn test_record() {
            let no_columns_nor_cell_values = Value::test_record(Record::new());

            let one_column_and_one_cell_value_with_empty_strings = Value::test_record(record! {
                "" => Value::test_string(""),
            });

            let one_column_with_a_string_and_one_cell_value_with_empty_string =
                Value::test_record(record! {
                    "column" => Value::test_string(""),
                });

            let one_column_with_empty_string_and_one_value_with_a_string =
                Value::test_record(record! {
                    "" => Value::test_string("text"),
                });

            assert!(no_columns_nor_cell_values.is_empty());
            assert!(!one_column_and_one_cell_value_with_empty_strings.is_empty());
            assert!(!one_column_with_a_string_and_one_cell_value_with_empty_string.is_empty());
            assert!(!one_column_with_empty_string_and_one_value_with_a_string.is_empty());
        }
    }

    mod get_type {
        use crate::Type;

        use super::*;

        #[test]
        fn test_list() {
            let list_of_ints = Value::test_list(vec![Value::test_int(0)]);
            let list_of_floats = Value::test_list(vec![Value::test_float(0.0)]);
            let list_of_ints_and_floats =
                Value::test_list(vec![Value::test_int(0), Value::test_float(0.0)]);
            let list_of_ints_and_floats_and_bools = Value::test_list(vec![
                Value::test_int(0),
                Value::test_float(0.0),
                Value::test_bool(false),
            ]);
            assert_eq!(list_of_ints.get_type(), Type::List(Box::new(Type::Int)));
            assert_eq!(list_of_floats.get_type(), Type::List(Box::new(Type::Float)));
            assert_eq!(
                list_of_ints_and_floats_and_bools.get_type(),
                Type::List(Box::new(Type::Any))
            );
            assert_eq!(
                list_of_ints_and_floats.get_type(),
                Type::List(Box::new(Type::Number))
            );
        }
    }

    mod into_string {
        use chrono::{DateTime, FixedOffset, NaiveDateTime};
        use rstest::rstest;

        use super::*;
        use crate::format_filesize;

        #[test]
        fn test_datetime() {
            let string = Value::test_date(DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_millis(-123456789).unwrap(),
                FixedOffset::east_opt(0).unwrap(),
            ))
            .into_string("", &Default::default());

            // We need to cut the humanized part off for tests to work, because
            // it is relative to current time.
            let formatted = string.split('(').next().unwrap();
            assert_eq!("Tue, 30 Dec 1969 13:42:23 +0000 ", formatted);
        }

        #[test]
        fn test_negative_year_datetime() {
            let string = Value::test_date(DateTime::from_naive_utc_and_offset(
                NaiveDateTime::from_timestamp_millis(-72135596800000).unwrap(),
                FixedOffset::east_opt(0).unwrap(),
            ))
            .into_string("", &Default::default());

            // We need to cut the humanized part off for tests to work, because
            // it is relative to current time.
            let formatted = string.split(' ').next().unwrap();
            assert_eq!("-0316-02-11T06:13:20+00:00", formatted);
        }

        #[rstest]
        #[case(1000, Some(true), "auto", "1.0 KB")]
        #[case(1000, Some(false), "auto", "1,000 B")]
        #[case(1000, Some(false), "kb", "1.0 KiB")]
        #[case(3000, Some(false), "auto", "2.9 KiB")]
        #[case(3_000_000, None, "auto", "2.9 MiB")]
        #[case(3_000_000, None, "kib", "2929.7 KiB")]
        fn test_filesize(
            #[case] val: i64,
            #[case] filesize_metric: Option<bool>,
            #[case] filesize_format: String,
            #[case] exp: &str,
        ) {
            assert_eq!(exp, format_filesize(val, &filesize_format, filesize_metric));
        }
    }
}
