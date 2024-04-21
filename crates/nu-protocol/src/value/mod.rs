mod custom_value;
mod duration;
mod filesize;
mod from;
mod from_value;
mod glob;
mod lazy_record;
mod range;

pub mod record;
pub use custom_value::CustomValue;
pub use duration::*;
pub use filesize::*;
pub use from_value::FromValue;
pub use glob::*;
pub use lazy_record::LazyRecord;
pub use range::{FloatRange, IntRange, Range};
pub use record::Record;

use crate::{
    ast::{Bits, Boolean, CellPath, Comparison, Math, Operator, PathMember},
    did_you_mean,
    engine::{Closure, EngineState},
    Config, ShellError, Span, Type,
};
use chrono::{DateTime, Datelike, FixedOffset, Locale, TimeZone};
use chrono_humanize::HumanTime;
use fancy_regex::Regex;
use nu_utils::{
    contains_emoji,
    locale::{get_system_locale_string, LOCALE_OVERRIDE_ENV_VAR},
    IgnoreCaseExt, SharedCow,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{Debug, Display, Write},
    ops::Bound,
    path::PathBuf,
};

/// Core structured values that pass through the pipeline in Nushell.
// NOTE: Please do not reorder these enum cases without thinking through the
// impact on the PartialOrd implementation and the global sort order
#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Bool {
        val: bool,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Int {
        val: i64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Float {
        val: f64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Filesize {
        val: i64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Duration {
        val: i64,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Date {
        val: DateTime<FixedOffset>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Range {
        val: Range,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    String {
        val: String,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Glob {
        val: String,
        no_expand: bool,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Record {
        val: SharedCow<Record>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    List {
        vals: Vec<Value>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Closure {
        val: Closure,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Nothing {
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Error {
        error: Box<ShellError>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Binary {
        val: Vec<u8>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    CellPath {
        val: CellPath,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Custom {
        val: Box<dyn CustomValue>,
        // note: spans are being refactored out of Value
        // please use .span() instead of matching this span value
        #[serde(rename = "span")]
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
                val: *val,
                internal_span: *internal_span,
            },
            Value::Float { val, internal_span } => Value::float(*val, *internal_span),
            Value::String { val, internal_span } => Value::String {
                val: val.clone(),
                internal_span: *internal_span,
            },
            Value::Glob {
                val,
                no_expand: quoted,
                internal_span,
            } => Value::Glob {
                val: val.clone(),
                no_expand: *quoted,
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
            Value::Custom { val, internal_span } => val.clone_value(*internal_span),
        }
    }
}

impl Value {
    fn cant_convert_to<T>(&self, typ: &str) -> Result<T, ShellError> {
        Err(ShellError::CantConvert {
            to_type: typ.into(),
            from_type: self.get_type().to_string(),
            span: self.span(),
            help: None,
        })
    }

    /// Returns the inner `bool` value or an error if this `Value` is not a bool
    pub fn as_bool(&self) -> Result<bool, ShellError> {
        if let Value::Bool { val, .. } = self {
            Ok(*val)
        } else {
            self.cant_convert_to("boolean")
        }
    }

    /// Returns the inner `i64` value or an error if this `Value` is not an int
    pub fn as_int(&self) -> Result<i64, ShellError> {
        if let Value::Int { val, .. } = self {
            Ok(*val)
        } else {
            self.cant_convert_to("int")
        }
    }

    /// Returns the inner `f64` value or an error if this `Value` is not a float
    pub fn as_float(&self) -> Result<f64, ShellError> {
        if let Value::Float { val, .. } = self {
            Ok(*val)
        } else {
            self.cant_convert_to("float")
        }
    }

    /// Returns this `Value` converted to a `f64` or an error if it cannot be converted
    ///
    /// Only the following `Value` cases will return an `Ok` result:
    /// - `Int`
    /// - `Float`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(val, Value::Float { .. } | Value::Int { .. }),
    ///         val.coerce_float().is_ok(),
    ///     );
    /// }
    /// ```
    pub fn coerce_float(&self) -> Result<f64, ShellError> {
        match self {
            Value::Float { val, .. } => Ok(*val),
            Value::Int { val, .. } => Ok(*val as f64),
            val => val.cant_convert_to("float"),
        }
    }

    /// Returns the inner `i64` filesize value or an error if this `Value` is not a filesize
    pub fn as_filesize(&self) -> Result<i64, ShellError> {
        if let Value::Filesize { val, .. } = self {
            Ok(*val)
        } else {
            self.cant_convert_to("filesize")
        }
    }

    /// Returns the inner `i64` duration value or an error if this `Value` is not a duration
    pub fn as_duration(&self) -> Result<i64, ShellError> {
        if let Value::Duration { val, .. } = self {
            Ok(*val)
        } else {
            self.cant_convert_to("duration")
        }
    }

    /// Returns the inner [`DateTime`] value or an error if this `Value` is not a date
    pub fn as_date(&self) -> Result<DateTime<FixedOffset>, ShellError> {
        if let Value::Date { val, .. } = self {
            Ok(*val)
        } else {
            self.cant_convert_to("date")
        }
    }

    /// Returns a reference to the inner [`Range`] value or an error if this `Value` is not a range
    pub fn as_range(&self) -> Result<Range, ShellError> {
        if let Value::Range { val, .. } = self {
            Ok(*val)
        } else {
            self.cant_convert_to("range")
        }
    }

    /// Unwraps the inner [`Range`] value or returns an error if this `Value` is not a range
    pub fn into_range(self) -> Result<Range, ShellError> {
        if let Value::Range { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("range")
        }
    }

    /// Returns a reference to the inner `str` value or an error if this `Value` is not a string
    pub fn as_str(&self) -> Result<&str, ShellError> {
        if let Value::String { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("string")
        }
    }

    /// Unwraps the inner `String` value or returns an error if this `Value` is not a string
    pub fn into_string(self) -> Result<String, ShellError> {
        if let Value::String { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("string")
        }
    }

    /// Returns this `Value` converted to a `str` or an error if it cannot be converted
    ///
    /// Only the following `Value` cases will return an `Ok` result:
    /// - `Int`
    /// - `Float`
    /// - `String`
    /// - `Binary` (only if valid utf-8)
    /// - `Date`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(
    ///             val,
    ///             Value::Int { .. }
    ///                 | Value::Float { .. }
    ///                 | Value::String { .. }
    ///                 | Value::Binary { .. }
    ///                 | Value::Date { .. }
    ///         ),
    ///         val.coerce_str().is_ok(),
    ///     );
    /// }
    /// ```
    pub fn coerce_str(&self) -> Result<Cow<str>, ShellError> {
        match self {
            Value::Int { val, .. } => Ok(Cow::Owned(val.to_string())),
            Value::Float { val, .. } => Ok(Cow::Owned(val.to_string())),
            Value::String { val, .. } => Ok(Cow::Borrowed(val)),
            Value::Binary { val, .. } => match std::str::from_utf8(val) {
                Ok(s) => Ok(Cow::Borrowed(s)),
                Err(_) => self.cant_convert_to("string"),
            },
            Value::Date { val, .. } => Ok(Cow::Owned(
                val.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            )),
            val => val.cant_convert_to("string"),
        }
    }

    /// Returns this `Value` converted to a `String` or an error if it cannot be converted
    ///
    /// # Note
    /// This function is equivalent to `value.coerce_str().map(Cow::into_owned)`
    /// which might allocate a new `String`.
    ///
    /// To avoid this allocation, prefer [`coerce_str`](Self::coerce_str)
    /// if you do not need an owned `String`,
    /// or [`coerce_into_string`](Self::coerce_into_string)
    /// if you do not need to keep the original `Value` around.
    ///
    /// Only the following `Value` cases will return an `Ok` result:
    /// - `Int`
    /// - `Float`
    /// - `String`
    /// - `Binary` (only if valid utf-8)
    /// - `Date`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(
    ///             val,
    ///             Value::Int { .. }
    ///                 | Value::Float { .. }
    ///                 | Value::String { .. }
    ///                 | Value::Binary { .. }
    ///                 | Value::Date { .. }
    ///         ),
    ///         val.coerce_string().is_ok(),
    ///     );
    /// }
    /// ```
    pub fn coerce_string(&self) -> Result<String, ShellError> {
        self.coerce_str().map(Cow::into_owned)
    }

    /// Returns this `Value` converted to a `String` or an error if it cannot be converted
    ///
    /// Only the following `Value` cases will return an `Ok` result:
    /// - `Int`
    /// - `Float`
    /// - `String`
    /// - `Binary` (only if valid utf-8)
    /// - `Date`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(
    ///             val,
    ///             Value::Int { .. }
    ///                 | Value::Float { .. }
    ///                 | Value::String { .. }
    ///                 | Value::Binary { .. }
    ///                 | Value::Date { .. }
    ///         ),
    ///         val.coerce_into_string().is_ok(),
    ///     );
    /// }
    /// ```
    pub fn coerce_into_string(self) -> Result<String, ShellError> {
        let span = self.span();
        match self {
            Value::Int { val, .. } => Ok(val.to_string()),
            Value::Float { val, .. } => Ok(val.to_string()),
            Value::String { val, .. } => Ok(val),
            Value::Binary { val, .. } => match String::from_utf8(val) {
                Ok(s) => Ok(s),
                Err(err) => Value::binary(err.into_bytes(), span).cant_convert_to("string"),
            },
            Value::Date { val, .. } => Ok(val.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
            val => val.cant_convert_to("string"),
        }
    }

    /// Returns this `Value` as a `char` or an error if it is not a single character string
    pub fn as_char(&self) -> Result<char, ShellError> {
        let span = self.span();
        if let Value::String { val, .. } = self {
            let mut chars = val.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Ok(c),
                _ => Err(ShellError::MissingParameter {
                    param_name: "single character separator".into(),
                    span,
                }),
            }
        } else {
            self.cant_convert_to("char")
        }
    }

    /// Converts this `Value` to a `PathBuf` or returns an error if it is not a string
    pub fn to_path(&self) -> Result<PathBuf, ShellError> {
        if let Value::String { val, .. } = self {
            Ok(PathBuf::from(val))
        } else {
            self.cant_convert_to("path")
        }
    }

    /// Returns a reference to the inner [`Record`] value or an error if this `Value` is not a record
    pub fn as_record(&self) -> Result<&Record, ShellError> {
        if let Value::Record { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("record")
        }
    }

    /// Unwraps the inner [`Record`] value or returns an error if this `Value` is not a record
    pub fn into_record(self) -> Result<Record, ShellError> {
        if let Value::Record { val, .. } = self {
            Ok(val.into_owned())
        } else {
            self.cant_convert_to("record")
        }
    }

    /// Returns a reference to the inner list slice or an error if this `Value` is not a list
    pub fn as_list(&self) -> Result<&[Value], ShellError> {
        if let Value::List { vals, .. } = self {
            Ok(vals)
        } else {
            self.cant_convert_to("list")
        }
    }

    /// Unwraps the inner list `Vec` or returns an error if this `Value` is not a list
    pub fn into_list(self) -> Result<Vec<Value>, ShellError> {
        if let Value::List { vals, .. } = self {
            Ok(vals)
        } else {
            self.cant_convert_to("list")
        }
    }

    /// Returns a reference to the inner [`Closure`] value or an error if this `Value` is not a closure
    pub fn as_closure(&self) -> Result<&Closure, ShellError> {
        if let Value::Closure { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("closure")
        }
    }

    /// Unwraps the inner [`Closure`] value or returns an error if this `Value` is not a closure
    pub fn into_closure(self) -> Result<Closure, ShellError> {
        if let Value::Closure { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("closure")
        }
    }

    /// Returns a reference to the inner binary slice or an error if this `Value` is not a binary value
    pub fn as_binary(&self) -> Result<&[u8], ShellError> {
        if let Value::Binary { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("binary")
        }
    }

    /// Unwraps the inner binary `Vec` or returns an error if this `Value` is not a binary value
    pub fn into_binary(self) -> Result<Vec<u8>, ShellError> {
        if let Value::Binary { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("binary")
        }
    }

    /// Returns this `Value` as a `u8` slice or an error if it cannot be converted
    ///
    /// Prefer [`coerce_into_binary`](Self::coerce_into_binary)
    /// if you do not need to keep the original `Value` around.
    ///
    /// Only the following `Value` cases will return an `Ok` result:
    /// - `Binary`
    /// - `String`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(val, Value::Binary { .. } | Value::String { .. }),
    ///         val.coerce_binary().is_ok(),
    ///     );
    /// }
    /// ```
    pub fn coerce_binary(&self) -> Result<&[u8], ShellError> {
        match self {
            Value::Binary { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(val.as_bytes()),
            val => val.cant_convert_to("binary"),
        }
    }

    /// Returns this `Value` as a `Vec<u8>` or an error if it cannot be converted
    ///
    /// Only the following `Value` cases will return an `Ok` result:
    /// - `Binary`
    /// - `String`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(val, Value::Binary { .. } | Value::String { .. }),
    ///         val.coerce_into_binary().is_ok(),
    ///     );
    /// }
    /// ```
    pub fn coerce_into_binary(self) -> Result<Vec<u8>, ShellError> {
        match self {
            Value::Binary { val, .. } => Ok(val),
            Value::String { val, .. } => Ok(val.into_bytes()),
            val => val.cant_convert_to("binary"),
        }
    }

    /// Returns a reference to the inner [`CellPath`] value or an error if this `Value` is not a cell path
    pub fn as_cell_path(&self) -> Result<&CellPath, ShellError> {
        if let Value::CellPath { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("cell path")
        }
    }

    /// Unwraps the inner [`CellPath`] value or returns an error if this `Value` is not a cell path
    pub fn into_cell_path(self) -> Result<CellPath, ShellError> {
        if let Value::CellPath { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("cell path")
        }
    }

    /// Returns a reference to the inner [`CustomValue`] trait object or an error if this `Value` is not a custom value
    pub fn as_custom_value(&self) -> Result<&dyn CustomValue, ShellError> {
        if let Value::Custom { val, .. } = self {
            Ok(val.as_ref())
        } else {
            self.cant_convert_to("custom value")
        }
    }

    /// Unwraps the inner [`CustomValue`] trait object or returns an error if this `Value` is not a custom value
    pub fn into_custom_value(self) -> Result<Box<dyn CustomValue>, ShellError> {
        if let Value::Custom { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("custom value")
        }
    }

    /// Returns a reference to the inner [`LazyRecord`] trait object or an error if this `Value` is not a lazy record
    pub fn as_lazy_record(&self) -> Result<&dyn for<'a> LazyRecord<'a>, ShellError> {
        if let Value::LazyRecord { val, .. } = self {
            Ok(val.as_ref())
        } else {
            self.cant_convert_to("lazy record")
        }
    }

    /// Unwraps the inner [`LazyRecord`] trait object or returns an error if this `Value` is not a lazy record
    pub fn into_lazy_record(self) -> Result<Box<dyn for<'a> LazyRecord<'a>>, ShellError> {
        if let Value::LazyRecord { val, .. } = self {
            Ok(val)
        } else {
            self.cant_convert_to("lazy record")
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
            | Value::Glob { internal_span, .. }
            | Value::Record { internal_span, .. }
            | Value::List { internal_span, .. }
            | Value::Closure { internal_span, .. }
            | Value::Nothing { internal_span, .. }
            | Value::Binary { internal_span, .. }
            | Value::CellPath { internal_span, .. }
            | Value::Custom { internal_span, .. }
            | Value::LazyRecord { internal_span, .. }
            | Value::Error { internal_span, .. } => *internal_span,
        }
    }

    /// Set the value's span to a new span
    pub fn set_span(&mut self, new_span: Span) {
        match self {
            Value::Bool { internal_span, .. }
            | Value::Int { internal_span, .. }
            | Value::Float { internal_span, .. }
            | Value::Filesize { internal_span, .. }
            | Value::Duration { internal_span, .. }
            | Value::Date { internal_span, .. }
            | Value::Range { internal_span, .. }
            | Value::String { internal_span, .. }
            | Value::Glob { internal_span, .. }
            | Value::Record { internal_span, .. }
            | Value::LazyRecord { internal_span, .. }
            | Value::List { internal_span, .. }
            | Value::Closure { internal_span, .. }
            | Value::Nothing { internal_span, .. }
            | Value::Binary { internal_span, .. }
            | Value::CellPath { internal_span, .. }
            | Value::Custom { internal_span, .. } => *internal_span = new_span,
            Value::Error { .. } => (),
        }
    }

    /// Update the value with a new span
    pub fn with_span(mut self, new_span: Span) -> Value {
        self.set_span(new_span);
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
            Value::Glob { .. } => Type::Glob,
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
            Value::Closure { .. } => Type::Closure,
            Value::Error { .. } => Type::Error,
            Value::Binary { .. } => Type::Binary,
            Value::CellPath { .. } => Type::CellPath,
            Value::Custom { val, .. } => Type::Custom(val.type_name().into()),
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

    fn format_datetime<Tz: TimeZone>(&self, date_time: &DateTime<Tz>, formatter: &str) -> String
    where
        Tz::Offset: Display,
    {
        let mut formatter_buf = String::new();
        let locale = if let Ok(l) =
            std::env::var(LOCALE_OVERRIDE_ENV_VAR).or_else(|_| std::env::var("LC_TIME"))
        {
            let locale_str = l.split('.').next().unwrap_or("en_US");
            locale_str.try_into().unwrap_or(Locale::en_US)
        } else {
            // LC_ALL > LC_CTYPE > LANG else en_US
            get_system_locale_string()
                .map(|l| l.replace('-', "_")) // `chrono::Locale` needs something like `xx_xx`, rather than `xx-xx`
                .unwrap_or_else(|| String::from("en_US"))
                .as_str()
                .try_into()
                .unwrap_or(Locale::en_US)
        };
        let format = date_time.format_localized(formatter, locale);

        match formatter_buf.write_fmt(format_args!("{format}")) {
            Ok(_) => (),
            Err(_) => formatter_buf = format!("Invalid format string {}", formatter),
        }
        formatter_buf
    }

    /// Converts this `Value` to a string according to the given [`Config`] and separator
    ///
    /// This functions recurses into records and lists,
    /// returning a string that contains the stringified form of all nested `Value`s.
    pub fn to_expanded_string(&self, separator: &str, config: &Config) -> String {
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
            Value::Range { val, .. } => val.to_string(),
            Value::String { val, .. } => val.clone(),
            Value::Glob { val, .. } => val.clone(),
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.to_expanded_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { val, .. } => format!(
                "{{{}}}",
                val.iter()
                    .map(|(x, y)| format!("{}: {}", x, y.to_expanded_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::LazyRecord { val, .. } => val
                .collect()
                .unwrap_or_else(|err| Value::error(err, span))
                .to_expanded_string(separator, config),
            Value::Closure { val, .. } => format!("<Closure {}>", val.block_id),
            Value::Nothing { .. } => String::new(),
            Value::Error { error, .. } => format!("{error:?}"),
            Value::Binary { val, .. } => format!("{val:?}"),
            Value::CellPath { val, .. } => val.to_string(),
            // If we fail to collapse the custom value, just print <{type_name}> - failure is not
            // that critical here
            Value::Custom { val, .. } => val
                .to_base_value(span)
                .map(|val| val.to_expanded_string(separator, config))
                .unwrap_or_else(|_| format!("<{}>", val.type_name())),
        }
    }

    /// Converts this `Value` to a string according to the given [`Config`]
    ///
    /// This functions does not recurse into records and lists.
    /// Instead, it will shorten the first list or record it finds like so:
    /// - "[table {n} rows]"
    /// - "[list {n} items]"
    /// - "[record {n} fields]"
    pub fn to_abbreviated_string(&self, config: &Config) -> String {
        let span = self.span();
        match self {
            Value::Date { val, .. } => match &config.datetime_table_format {
                Some(format) => self.format_datetime(val, format),
                None => HumanTime::from(*val).to_string(),
            },
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
            Value::LazyRecord { val, .. } => val
                .collect()
                .unwrap_or_else(|err| Value::error(err, span))
                .to_abbreviated_string(config),
            val => val.to_expanded_string(", ", config),
        }
    }

    /// Converts this `Value` to a string according to the given [`Config`] and separator
    ///
    /// This function adds quotes around strings,
    /// so that the returned string can be parsed by nushell.
    /// The other `Value` cases are already parsable when converted strings
    /// or are not yet handled by this function.
    ///
    /// This functions behaves like [`to_expanded_string`](Self::to_expanded_string)
    /// and will recurse into records and lists.
    pub fn to_parsable_string(&self, separator: &str, config: &Config) -> String {
        match self {
            // give special treatment to the simple types to make them parsable
            Value::String { val, .. } => format!("'{}'", val),
            // recurse back into this function for recursive formatting
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.iter()
                    .map(|x| x.to_parsable_string(", ", config))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            Value::Record { val, .. } => format!(
                "{{{}}}",
                val.iter()
                    .map(|(x, y)| format!("{}: {}", x, y.to_parsable_string(", ", config)))
                    .collect::<Vec<_>>()
                    .join(separator)
            ),
            // defer to standard handling for types where standard representation is parsable
            _ => self.to_expanded_string(separator, config),
        }
    }

    /// Convert this `Value` to a debug string
    ///
    /// In general, this function should only be used for debug purposes,
    /// and the resulting string should not be displayed to the user (not even in an error).
    pub fn to_debug_string(&self) -> String {
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
                            if let Some(item) =
                                val.into_range_iter(current.span(), None).nth(*count)
                            {
                                current = item;
                            } else if *optional {
                                return Ok(Value::nothing(*origin_span)); // short-circuit
                            } else {
                                return Err(ShellError::AccessBeyondEndOfStream {
                                    span: *origin_span,
                                });
                            }
                        }
                        Value::Custom { ref val, .. } => {
                            current =
                                match val.follow_path_int(current.span(), *count, *origin_span) {
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
                        Value::Record { .. } | Value::LazyRecord { .. } => {
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
                        Value::Record { mut val, .. } => {
                            // Make reverse iterate to avoid duplicate column leads to first value, actually last value is expected.
                            if let Some(found) = val.to_mut().iter_mut().rev().find(|x| {
                                if insensitive {
                                    x.0.eq_ignore_case(column_name)
                                } else {
                                    x.0 == column_name
                                }
                            }) {
                                current = std::mem::take(found.1);
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
                                        Value::Record { mut val, .. } => {
                                            if let Some(found) =
                                                val.to_mut().iter_mut().rev().find(|x| {
                                                    if insensitive {
                                                        x.0.eq_ignore_case(column_name)
                                                    } else {
                                                        x.0 == column_name
                                                    }
                                                })
                                            {
                                                Ok(std::mem::take(found.1))
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
                        Value::Custom { ref val, .. } => {
                            current = match val.follow_path_string(
                                current.span(),
                                column_name.clone(),
                                *origin_span,
                            ) {
                                Ok(val) => val,
                                Err(err) => {
                                    if *optional {
                                        return Ok(Value::nothing(*origin_span));
                                    // short-circuit
                                    } else {
                                        return Err(err);
                                    }
                                }
                            }
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
                                    if let Some(val) = record.to_mut().get_mut(col_name) {
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
                                        record.to_mut().push(col_name, new_col);
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
                        if let Some(val) = record.to_mut().get_mut(col_name) {
                            val.upsert_data_at_cell_path(path, new_val)?;
                        } else {
                            let new_col = if path.is_empty() {
                                new_val
                            } else {
                                let mut new_col = Value::record(Record::new(), new_val.span());
                                new_col.upsert_data_at_cell_path(path, new_val)?;
                                new_col
                            };
                            record.to_mut().push(col_name, new_col);
                        }
                    }
                    Value::LazyRecord { val, .. } => {
                        // convert to Record first.
                        *self = val.collect()?;
                        self.upsert_data_at_cell_path(cell_path, new_val)?;
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
                                    if let Some(val) = record.to_mut().get_mut(col_name) {
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
                        if let Some(val) = record.to_mut().get_mut(col_name) {
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
                        *self = val.collect()?;
                        self.update_data_at_cell_path(cell_path, new_val)?;
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
                                        if record.to_mut().remove(col_name).is_none() && !optional {
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
                            if record.to_mut().remove(col_name).is_none() && !optional {
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
                            *self = val.collect()?;
                            self.remove_data_at_cell_path(cell_path)
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
                                        if let Some(val) = record.to_mut().get_mut(col_name) {
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
                            if let Some(val) = record.to_mut().get_mut(col_name) {
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
                            *self = val.collect()?;
                            self.remove_data_at_cell_path(cell_path)
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
                                    if let Some(val) = record.to_mut().get_mut(col_name) {
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
                                        record.to_mut().push(col_name, new_col);
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
                        if let Some(val) = record.to_mut().get_mut(col_name) {
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
                                new_val
                            } else {
                                let mut new_col = Value::record(Record::new(), new_val.span());
                                new_col.insert_data_at_cell_path(path, new_val, head_span)?;
                                new_col
                            };
                            record.to_mut().push(col_name, new_col);
                        }
                    }
                    Value::LazyRecord { val, .. } => {
                        // convert to Record first.
                        *self = val.collect()?;
                        self.insert_data_at_cell_path(cell_path, new_val, v_span)?;
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

    /// Visits all values contained within the value (including this value) with a mutable reference
    /// given to the closure.
    ///
    /// If the closure returns `Err`, the traversal will stop.
    ///
    /// If collecting lazy records to check them as well is desirable, make sure to do it in your
    /// closure. The traversal continues on whatever modifications you make during the closure.
    /// Captures of closure values are currently visited, as they are values owned by the closure.
    pub fn recurse_mut<E>(
        &mut self,
        f: &mut impl FnMut(&mut Value) -> Result<(), E>,
    ) -> Result<(), E> {
        // Visit this value
        f(self)?;
        // Check for contained values
        match self {
            Value::Record { ref mut val, .. } => val
                .to_mut()
                .iter_mut()
                .try_for_each(|(_, rec_value)| rec_value.recurse_mut(f)),
            Value::List { ref mut vals, .. } => vals
                .iter_mut()
                .try_for_each(|list_value| list_value.recurse_mut(f)),
            // Closure captures are visited. Maybe these don't have to be if they are changed to
            // more opaque references.
            Value::Closure { ref mut val, .. } => val
                .captures
                .iter_mut()
                .map(|(_, captured_value)| captured_value)
                .try_for_each(|captured_value| captured_value.recurse_mut(f)),
            // All of these don't contain other values
            Value::Bool { .. }
            | Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. }
            | Value::Date { .. }
            | Value::Range { .. }
            | Value::String { .. }
            | Value::Glob { .. }
            | Value::Nothing { .. }
            | Value::Error { .. }
            | Value::Binary { .. }
            | Value::CellPath { .. } => Ok(()),
            // These could potentially contain values, but we expect the closure to handle them
            Value::LazyRecord { .. } | Value::Custom { .. } => Ok(()),
        }
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
            val,
            internal_span: span,
        }
    }

    pub fn string(val: impl Into<String>, span: Span) -> Value {
        Value::String {
            val: val.into(),
            internal_span: span,
        }
    }

    pub fn glob(val: impl Into<String>, no_expand: bool, span: Span) -> Value {
        Value::Glob {
            val: val.into(),
            no_expand,
            internal_span: span,
        }
    }

    pub fn record(val: Record, span: Span) -> Value {
        Value::Record {
            val: SharedCow::new(val),
            internal_span: span,
        }
    }

    pub fn list(vals: Vec<Value>, span: Span) -> Value {
        Value::List {
            vals,
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

    pub fn custom(val: Box<dyn CustomValue>, span: Span) -> Value {
        Value::Custom {
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
    pub fn test_glob(val: impl Into<String>) -> Value {
        Value::glob(val, false, Span::test_data())
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
        Value::custom(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data, as it will point into unknown source
    /// when used in errors.
    pub fn test_lazy_record(val: Box<dyn for<'a> LazyRecord<'a>>) -> Value {
        Value::lazy_record(val, Span::test_data())
    }

    /// Note: Only use this for test data, *not* live data,
    /// as it will point into unknown source when used in errors.
    ///
    /// Returns a `Vec` containing one of each value case (`Value::Int`, `Value::String`, etc.)
    /// except for `Value::LazyRecord` and `Value::CustomValue`.
    pub fn test_values() -> Vec<Value> {
        vec![
            Value::test_bool(false),
            Value::test_int(0),
            Value::test_filesize(0),
            Value::test_duration(0),
            Value::test_date(DateTime::UNIX_EPOCH.into()),
            Value::test_range(Range::IntRange(IntRange {
                start: 0,
                step: 1,
                end: Bound::Excluded(0),
            })),
            Value::test_float(0.0),
            Value::test_string(String::new()),
            Value::test_record(Record::new()),
            // Value::test_lazy_record(Box::new(todo!())),
            Value::test_list(Vec::new()),
            Value::test_closure(Closure {
                block_id: 0,
                captures: Vec::new(),
            }),
            Value::test_nothing(),
            Value::error(
                ShellError::NushellFailed { msg: String::new() },
                Span::test_data(),
            ),
            Value::test_binary(Vec::new()),
            Value::test_cell_path(CellPath {
                members: Vec::new(),
            }),
            // Value::test_custom_value(Box::new(todo!())),
        ]
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
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
            },
            (Value::Glob { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::String { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Glob { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Record { .. } => Some(Ordering::Less),
                Value::LazyRecord { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Record { val: rhs, .. } => {
                    // reorder cols and vals to make more logically compare.
                    // more general, if two record have same col and values,
                    // the order of cols shouldn't affect the equal property.
                    let mut lhs = lhs.clone().into_owned();
                    let mut rhs = rhs.clone().into_owned();
                    lhs.sort_cols();
                    rhs.sort_cols();

                    // Check columns first
                    for (a, b) in lhs.columns().zip(rhs.columns()) {
                        let result = a.partial_cmp(b);
                        if result != Some(Ordering::Equal) {
                            return result;
                        }
                    }
                    // Then check the values
                    for (a, b) in lhs.values().zip(rhs.values()) {
                        let result = a.partial_cmp(b);
                        if result != Some(Ordering::Equal) {
                            return result;
                        }
                    }
                    // If all of the comparisons were equal, then lexicographical order dictates
                    // that the shorter sequence is less than the longer one
                    lhs.len().partial_cmp(&rhs.len())
                }
                Value::LazyRecord { val, .. } => {
                    if let Ok(rhs) = val.collect() {
                        self.partial_cmp(&rhs)
                    } else {
                        None
                    }
                }
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { vals: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { val: rhs, .. } => lhs.block_id.partial_cmp(&rhs.block_id),
                Value::Nothing { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Equal),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Equal),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
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
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::LazyRecord { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { .. } => Some(Ordering::Greater),
                Value::CellPath { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Custom { .. } => Some(Ordering::Less),
            },
            (Value::Custom { val: lhs, .. }, rhs) => lhs.partial_cmp(rhs),
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

            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Append), op, rhs)
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

            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
        if let (Value::Custom { val: lhs, .. }, rhs) = (self, rhs) {
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
        if let (Value::Custom { val: lhs, .. }, rhs) = (self, rhs) {
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
        if let (Value::Custom { val: lhs, .. }, rhs) = (self, rhs) {
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
        if let (Value::Custom { val: lhs, .. }, rhs) = (self, rhs) {
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
        if let (Value::Custom { val: lhs, .. }, rhs) = (self, rhs) {
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
        if let (Value::Custom { val: lhs, .. }, rhs) = (self, rhs) {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => lhs.operation(
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
            (Value::Custom { val: lhs, .. }, rhs) => lhs.operation(
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
            (Value::Custom { val: lhs, .. }, rhs) => lhs.operation(
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
            (Value::Custom { val: lhs, .. }, rhs) => lhs.operation(
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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
            (Value::Custom { val: lhs, .. }, rhs) => {
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

// TODO: The name of this function is overly broad with partial compatibility
// Should be replaced by an explicitly named helper on `Type` (take `Any` into account)
fn type_compatible(a: Type, b: Type) -> bool {
    if a == b {
        return true;
    }

    matches!((a, b), (Type::Int, Type::Float) | (Type::Float, Type::Int))
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
        use chrono::{DateTime, FixedOffset};

        use super::*;

        #[test]
        fn test_datetime() {
            let date = DateTime::from_timestamp_millis(-123456789)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap());

            let string = Value::test_date(date).to_expanded_string("", &Default::default());

            // We need to cut the humanized part off for tests to work, because
            // it is relative to current time.
            let formatted = string.split('(').next().unwrap();
            assert_eq!("Tue, 30 Dec 1969 13:42:23 +0000 ", formatted);
        }

        #[test]
        fn test_negative_year_datetime() {
            let date = DateTime::from_timestamp_millis(-72135596800000)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap());

            let string = Value::test_date(date).to_expanded_string("", &Default::default());

            // We need to cut the humanized part off for tests to work, because
            // it is relative to current time.
            let formatted = string.split(' ').next().unwrap();
            assert_eq!("-0316-02-11T06:13:20+00:00", formatted);
        }
    }
}
