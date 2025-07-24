mod custom_value;
mod duration;
mod filesize;
mod from_value;
mod glob;
mod into_value;
mod range;
#[cfg(test)]
mod test_derive;

pub mod record;
pub use custom_value::CustomValue;
pub use duration::*;
pub use filesize::*;
pub use from_value::FromValue;
pub use glob::*;
pub use into_value::{IntoValue, TryIntoValue};
pub use range::{FloatRange, IntRange, Range};
pub use record::Record;

use crate::{
    BlockId, Config, ShellError, Signals, Span, Type,
    ast::{Bits, Boolean, CellPath, Comparison, Math, Operator, PathMember},
    did_you_mean,
    engine::{Closure, EngineState},
};
use chrono::{DateTime, Datelike, Duration, FixedOffset, Local, Locale, TimeZone};
use chrono_humanize::HumanTime;
use fancy_regex::Regex;
use nu_utils::{
    ObviousFloat, SharedCow, contains_emoji,
    locale::{LOCALE_OVERRIDE_ENV_VAR, get_system_locale_string},
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{Debug, Display, Write},
    ops::{Bound, ControlFlow, Deref},
    path::PathBuf,
};

/// Core structured values that pass through the pipeline in Nushell.
// NOTE: Please do not reorder these enum cases without thinking through the
// impact on the PartialOrd implementation and the global sort order
#[derive(Debug, Serialize, Deserialize)]
pub enum Value {
    Bool {
        val: bool,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Int {
        val: i64,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Float {
        val: f64,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    String {
        val: String,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Glob {
        val: String,
        no_expand: bool,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Filesize {
        val: Filesize,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Duration {
        val: i64,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Date {
        val: DateTime<FixedOffset>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Range {
        val: Box<Range>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Record {
        val: SharedCow<Record>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    List {
        vals: Vec<Value>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Closure {
        val: Box<Closure>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Error {
        error: Box<ShellError>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Binary {
        val: Vec<u8>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    CellPath {
        val: CellPath,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Custom {
        val: Box<dyn CustomValue>,
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
    Nothing {
        /// note: spans are being refactored out of Value
        /// please use .span() instead of matching this span value
        #[serde(rename = "span")]
        internal_span: Span,
    },
}

// This is to document/enforce the size of `Value` in bytes.
// We should try to avoid increasing the size of `Value`,
// and PRs that do so will have to change the number below so that it's noted in review.
const _: () = assert!(std::mem::size_of::<Value>() <= 48);

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
    pub fn as_filesize(&self) -> Result<Filesize, ShellError> {
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
            self.cant_convert_to("datetime")
        }
    }

    /// Returns a reference to the inner [`Range`] value or an error if this `Value` is not a range
    pub fn as_range(&self) -> Result<Range, ShellError> {
        if let Value::Range { val, .. } = self {
            Ok(**val)
        } else {
            self.cant_convert_to("range")
        }
    }

    /// Unwraps the inner [`Range`] value or returns an error if this `Value` is not a range
    pub fn into_range(self) -> Result<Range, ShellError> {
        if let Value::Range { val, .. } = self {
            Ok(*val)
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
    /// - `Bool`
    /// - `Int`
    /// - `Float`
    /// - `String`
    /// - `Glob`
    /// - `Binary` (only if valid utf-8)
    /// - `Date`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(
    ///             val,
    ///             Value::Bool { .. }
    ///                 | Value::Int { .. }
    ///                 | Value::Float { .. }
    ///                 | Value::String { .. }
    ///                 | Value::Glob { .. }
    ///                 | Value::Binary { .. }
    ///                 | Value::Date { .. }
    ///         ),
    ///         val.coerce_str().is_ok(),
    ///     );
    /// }
    /// ```
    pub fn coerce_str(&self) -> Result<Cow<str>, ShellError> {
        match self {
            Value::Bool { val, .. } => Ok(Cow::Owned(val.to_string())),
            Value::Int { val, .. } => Ok(Cow::Owned(val.to_string())),
            Value::Float { val, .. } => Ok(Cow::Owned(val.to_string())),
            Value::String { val, .. } => Ok(Cow::Borrowed(val)),
            Value::Glob { val, .. } => Ok(Cow::Borrowed(val)),
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
    /// - `Bool`
    /// - `Int`
    /// - `Float`
    /// - `String`
    /// - `Glob`
    /// - `Binary` (only if valid utf-8)
    /// - `Date`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(
    ///             val,
    ///             Value::Bool { .. }
    ///                 | Value::Int { .. }
    ///                 | Value::Float { .. }
    ///                 | Value::String { .. }
    ///                 | Value::Glob { .. }
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
    /// - `Bool`
    /// - `Int`
    /// - `Float`
    /// - `String`
    /// - `Glob`
    /// - `Binary` (only if valid utf-8)
    /// - `Date`
    ///
    /// ```
    /// # use nu_protocol::Value;
    /// for val in Value::test_values() {
    ///     assert_eq!(
    ///         matches!(
    ///             val,
    ///             Value::Bool { .. }
    ///                 | Value::Int { .. }
    ///                 | Value::Float { .. }
    ///                 | Value::String { .. }
    ///                 | Value::Glob { .. }
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
            Value::Bool { val, .. } => Ok(val.to_string()),
            Value::Int { val, .. } => Ok(val.to_string()),
            Value::Float { val, .. } => Ok(val.to_string()),
            Value::String { val, .. } => Ok(val),
            Value::Glob { val, .. } => Ok(val),
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
            Ok(*val)
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

    /// Interprets this `Value` as a boolean based on typical conventions for environment values.
    ///
    /// The following rules are used:
    /// - Values representing `false`:
    ///   - Empty strings or strings that equal to "false" in any case
    ///   - The number `0` (as an integer, float or string)
    ///   - `Nothing`
    ///   - Explicit boolean `false`
    /// - Values representing `true`:
    ///   - Non-zero numbers (integer or float)
    ///   - Non-empty strings
    ///   - Explicit boolean `true`
    ///
    /// For all other, more complex variants of [`Value`], the function cannot determine a
    /// boolean representation and returns `Err`.
    pub fn coerce_bool(&self) -> Result<bool, ShellError> {
        match self {
            Value::Bool { val: false, .. } | Value::Int { val: 0, .. } | Value::Nothing { .. } => {
                Ok(false)
            }
            Value::Float { val, .. } if val <= &f64::EPSILON => Ok(false),
            Value::String { val, .. } => match val.trim().to_ascii_lowercase().as_str() {
                "" | "0" | "false" => Ok(false),
                _ => Ok(true),
            },
            Value::Bool { .. } | Value::Int { .. } | Value::Float { .. } => Ok(true),
            _ => self.cant_convert_to("bool"),
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
            Value::Nothing { .. } => Type::Nothing,
            Value::Closure { .. } => Type::Closure,
            Value::Error { .. } => Type::Error,
            Value::Binary { .. } => Type::Binary,
            Value::CellPath { .. } => Type::CellPath,
            Value::Custom { val, .. } => Type::Custom(val.type_name().into()),
        }
    }

    /// Determine of the [`Value`] is a [subtype](https://en.wikipedia.org/wiki/Subtyping) of `other`
    ///
    /// If you have a [`Value`], this method should always be used over chaining [`Value::get_type`] with [`Type::is_subtype_of`](crate::Type::is_subtype_of).
    ///
    /// This method is able to leverage that information encoded in a `Value` to provide more accurate
    /// type comparison than if one were to collect the type into [`Type`](crate::Type) value with [`Value::get_type`].
    ///
    /// Empty lists are considered subtypes of all `list<T>` types.
    ///
    /// Lists of mixed records where some column is present in all record is a subtype of `table<column>`.
    /// For example, `[{a: 1, b: 2}, {a: 1}]` is a subtype of `table<a: int>` (but not `table<a: int, b: int>`).
    ///
    /// See also: [`PipelineData::is_subtype_of`](crate::PipelineData::is_subtype_of)
    pub fn is_subtype_of(&self, other: &Type) -> bool {
        // records are structurally typed
        let record_compatible = |val: &Value, other: &[(String, Type)]| match val {
            Value::Record { val, .. } => other
                .iter()
                .all(|(key, ty)| val.get(key).is_some_and(|inner| inner.is_subtype_of(ty))),
            _ => false,
        };

        // All cases matched explicitly to ensure this does not accidentally allocate `Type` if any composite types are introduced in the future
        match (self, other) {
            (_, Type::Any) => true,

            // `Type` allocation for scalar types is trivial
            (
                Value::Bool { .. }
                | Value::Int { .. }
                | Value::Float { .. }
                | Value::String { .. }
                | Value::Glob { .. }
                | Value::Filesize { .. }
                | Value::Duration { .. }
                | Value::Date { .. }
                | Value::Range { .. }
                | Value::Closure { .. }
                | Value::Error { .. }
                | Value::Binary { .. }
                | Value::CellPath { .. }
                | Value::Nothing { .. },
                _,
            ) => self.get_type().is_subtype_of(other),

            // matching composite types
            (val @ Value::Record { .. }, Type::Record(inner)) => record_compatible(val, inner),
            (Value::List { vals, .. }, Type::List(inner)) => {
                vals.iter().all(|val| val.is_subtype_of(inner))
            }
            (Value::List { vals, .. }, Type::Table(inner)) => {
                vals.iter().all(|val| record_compatible(val, inner))
            }
            (Value::Custom { val, .. }, Type::Custom(inner)) => val.type_name() == **inner,

            // non-matching composite types
            (Value::Record { .. } | Value::List { .. } | Value::Custom { .. }, _) => false,
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
            Err(_) => formatter_buf = format!("Invalid format string {formatter}"),
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
            Value::Float { val, .. } => ObviousFloat(*val).to_string(),
            Value::Filesize { val, .. } => config.filesize.format(*val).to_string(),
            Value::Duration { val, .. } => format_duration(*val),
            Value::Date { val, .. } => match &config.datetime_format.normal {
                Some(format) => self.format_datetime(val, format),
                None => {
                    format!(
                        "{} ({})",
                        if val.year() >= 0 {
                            val.to_rfc2822()
                        } else {
                            val.to_rfc3339()
                        },
                        human_time_from_now(val),
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
            Value::Closure { val, .. } => format!("closure_{}", val.block_id.get()),
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
        match self {
            Value::Date { val, .. } => match &config.datetime_format.table {
                Some(format) => self.format_datetime(val, format),
                None => human_time_from_now(val).to_string(),
            },
            Value::List { vals, .. } => {
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
            Value::String { val, .. } => format!("'{val}'"),
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
    pub fn follow_cell_path<'out>(
        &'out self,
        cell_path: &[PathMember],
    ) -> Result<Cow<'out, Value>, ShellError> {
        enum MultiLife<'out, 'local, T>
        where
            'out: 'local,
            T: ?Sized,
        {
            Out(&'out T),
            Local(&'local T),
        }

        impl<'out, 'local, T> Deref for MultiLife<'out, 'local, T>
        where
            'out: 'local,
            T: ?Sized,
        {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                match *self {
                    MultiLife::Out(x) => x,
                    MultiLife::Local(x) => x,
                }
            }
        }

        // A dummy value is required, otherwise rust doesn't allow references, which we need for
        // the `std::ptr::eq` comparison
        let mut store: Value = Value::test_nothing();
        let mut current: MultiLife<'out, '_, Value> = MultiLife::Out(self);

        let reorder_cell_paths = nu_experimental::REORDER_CELL_PATHS.get();

        let mut members: Vec<_> = if reorder_cell_paths {
            cell_path.iter().map(Some).collect()
        } else {
            Vec::new()
        };
        let mut members = members.as_mut_slice();
        let mut cell_path = cell_path;

        loop {
            let member = if reorder_cell_paths {
                // Skip any None values at the start.
                while let Some(None) = members.first() {
                    members = &mut members[1..];
                }

                if members.is_empty() {
                    break;
                }

                // Reorder cell-path member access by prioritizing Int members to avoid cloning unless
                // necessary
                let member = if let Value::List { .. } = &*current {
                    // If the value is a list, try to find an Int member
                    members
                        .iter_mut()
                        .find(|x| matches!(x, Some(PathMember::Int { .. })))
                        // And take it from the list of members
                        .and_then(Option::take)
                } else {
                    None
                };

                let Some(member) = member.or_else(|| members.first_mut().and_then(Option::take))
                else {
                    break;
                };
                member
            } else {
                match cell_path {
                    [first, rest @ ..] => {
                        cell_path = rest;
                        first
                    }
                    _ => break,
                }
            };

            current = match current {
                MultiLife::Out(current) => match get_value_member(current, member)? {
                    ControlFlow::Break(span) => return Ok(Cow::Owned(Value::nothing(span))),
                    ControlFlow::Continue(x) => match x {
                        Cow::Borrowed(x) => MultiLife::Out(x),
                        Cow::Owned(x) => {
                            store = x;
                            MultiLife::Local(&store)
                        }
                    },
                },
                MultiLife::Local(current) => match get_value_member(current, member)? {
                    ControlFlow::Break(span) => return Ok(Cow::Owned(Value::nothing(span))),
                    ControlFlow::Continue(x) => match x {
                        Cow::Borrowed(x) => MultiLife::Local(x),
                        Cow::Owned(x) => {
                            store = x;
                            MultiLife::Local(&store)
                        }
                    },
                },
            };
        }

        // If a single Value::Error was produced by the above (which won't happen if nullify_errors is true), unwrap it now.
        // Note that Value::Errors inside Lists remain as they are, so that the rest of the list can still potentially be used.
        if let Value::Error { error, .. } = &*current {
            Err(error.as_ref().clone())
        } else {
            Ok(match current {
                MultiLife::Out(x) => Cow::Borrowed(x),
                MultiLife::Local(x) => {
                    let x = if std::ptr::eq(x, &store) {
                        store
                    } else {
                        x.clone()
                    };
                    Cow::Owned(x)
                }
            })
        }
    }

    /// Follow a given cell path into the value: for example accessing select elements in a stream or list
    pub fn upsert_cell_path(
        &mut self,
        cell_path: &[PathMember],
        callback: Box<dyn FnOnce(&Value) -> Value>,
    ) -> Result<(), ShellError> {
        let new_val = callback(self.follow_cell_path(cell_path)?.as_ref());

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
                    casing,
                    ..
                } => match self {
                    Value::List { vals, .. } => {
                        for val in vals.iter_mut() {
                            match val {
                                Value::Record { val: record, .. } => {
                                    let record = record.to_mut();
                                    if let Some(val) = record.cased_mut(*casing).get_mut(col_name) {
                                        val.upsert_data_at_cell_path(path, new_val.clone())?;
                                    } else {
                                        let new_col =
                                            Value::with_data_at_cell_path(path, new_val.clone())?;
                                        record.push(col_name, new_col);
                                    }
                                }
                                Value::Error { error, .. } => return Err(*error.clone()),
                                v => {
                                    return Err(ShellError::CantFindColumn {
                                        col_name: col_name.clone(),
                                        span: Some(*span),
                                        src_span: v.span(),
                                    });
                                }
                            }
                        }
                    }
                    Value::Record { val: record, .. } => {
                        let record = record.to_mut();
                        if let Some(val) = record.cased_mut(*casing).get_mut(col_name) {
                            val.upsert_data_at_cell_path(path, new_val)?;
                        } else {
                            let new_col = Value::with_data_at_cell_path(path, new_val.clone())?;
                            record.push(col_name, new_col);
                        }
                    }
                    Value::Error { error, .. } => return Err(*error.clone()),
                    v => {
                        return Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: Some(*span),
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
                        } else {
                            // If the upsert is at 1 + the end of the list, it's OK.
                            vals.push(Value::with_data_at_cell_path(path, new_val)?);
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
        let new_val = callback(self.follow_cell_path(cell_path)?.as_ref());

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
                    casing,
                    ..
                } => match self {
                    Value::List { vals, .. } => {
                        for val in vals.iter_mut() {
                            let v_span = val.span();
                            match val {
                                Value::Record { val: record, .. } => {
                                    if let Some(val) =
                                        record.to_mut().cased_mut(*casing).get_mut(col_name)
                                    {
                                        val.update_data_at_cell_path(path, new_val.clone())?;
                                    } else {
                                        return Err(ShellError::CantFindColumn {
                                            col_name: col_name.clone(),
                                            span: Some(*span),
                                            src_span: v_span,
                                        });
                                    }
                                }
                                Value::Error { error, .. } => return Err(*error.clone()),
                                v => {
                                    return Err(ShellError::CantFindColumn {
                                        col_name: col_name.clone(),
                                        span: Some(*span),
                                        src_span: v.span(),
                                    });
                                }
                            }
                        }
                    }
                    Value::Record { val: record, .. } => {
                        if let Some(val) = record.to_mut().cased_mut(*casing).get_mut(col_name) {
                            val.update_data_at_cell_path(path, new_val)?;
                        } else {
                            return Err(ShellError::CantFindColumn {
                                col_name: col_name.clone(),
                                span: Some(*span),
                                src_span: v_span,
                            });
                        }
                    }
                    Value::Error { error, .. } => return Err(*error.clone()),
                    v => {
                        return Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: Some(*span),
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
                        casing,
                    } => match self {
                        Value::List { vals, .. } => {
                            for val in vals.iter_mut() {
                                let v_span = val.span();
                                match val {
                                    Value::Record { val: record, .. } => {
                                        let value =
                                            record.to_mut().cased_mut(*casing).remove(col_name);
                                        if value.is_none() && !optional {
                                            return Err(ShellError::CantFindColumn {
                                                col_name: col_name.clone(),
                                                span: Some(*span),
                                                src_span: v_span,
                                            });
                                        }
                                    }
                                    v => {
                                        return Err(ShellError::CantFindColumn {
                                            col_name: col_name.clone(),
                                            span: Some(*span),
                                            src_span: v.span(),
                                        });
                                    }
                                }
                            }
                            Ok(())
                        }
                        Value::Record { val: record, .. } => {
                            if record
                                .to_mut()
                                .cased_mut(*casing)
                                .remove(col_name)
                                .is_none()
                                && !optional
                            {
                                return Err(ShellError::CantFindColumn {
                                    col_name: col_name.clone(),
                                    span: Some(*span),
                                    src_span: v_span,
                                });
                            }
                            Ok(())
                        }
                        v => Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: Some(*span),
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
                        casing,
                    } => match self {
                        Value::List { vals, .. } => {
                            for val in vals.iter_mut() {
                                let v_span = val.span();
                                match val {
                                    Value::Record { val: record, .. } => {
                                        let val =
                                            record.to_mut().cased_mut(*casing).get_mut(col_name);
                                        if let Some(val) = val {
                                            val.remove_data_at_cell_path(path)?;
                                        } else if !optional {
                                            return Err(ShellError::CantFindColumn {
                                                col_name: col_name.clone(),
                                                span: Some(*span),
                                                src_span: v_span,
                                            });
                                        }
                                    }
                                    v => {
                                        return Err(ShellError::CantFindColumn {
                                            col_name: col_name.clone(),
                                            span: Some(*span),
                                            src_span: v.span(),
                                        });
                                    }
                                }
                            }
                            Ok(())
                        }
                        Value::Record { val: record, .. } => {
                            if let Some(val) = record.to_mut().cased_mut(*casing).get_mut(col_name)
                            {
                                val.remove_data_at_cell_path(path)?;
                            } else if !optional {
                                return Err(ShellError::CantFindColumn {
                                    col_name: col_name.clone(),
                                    span: Some(*span),
                                    src_span: v_span,
                                });
                            }
                            Ok(())
                        }
                        v => Err(ShellError::CantFindColumn {
                            col_name: col_name.clone(),
                            span: Some(*span),
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
                    casing,
                    ..
                } => match self {
                    Value::List { vals, .. } => {
                        for val in vals.iter_mut() {
                            let v_span = val.span();
                            match val {
                                Value::Record { val: record, .. } => {
                                    let record = record.to_mut();
                                    if let Some(val) = record.cased_mut(*casing).get_mut(col_name) {
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
                                        let new_col =
                                            Value::with_data_at_cell_path(path, new_val.clone())?;
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
                        let record = record.to_mut();
                        if let Some(val) = record.cased_mut(*casing).get_mut(col_name) {
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
                            let new_col = Value::with_data_at_cell_path(path, new_val)?;
                            record.push(col_name, new_col);
                        }
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
                        } else {
                            // If the insert is at 1 + the end of the list, it's OK.
                            vals.push(Value::with_data_at_cell_path(path, new_val)?);
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

    /// Creates a new [Value] with the specified member at the specified path.
    /// This is used by [Value::insert_data_at_cell_path] and [Value::upsert_data_at_cell_path] whenever they have the need to insert a non-existent element
    fn with_data_at_cell_path(cell_path: &[PathMember], value: Value) -> Result<Value, ShellError> {
        if let Some((member, path)) = cell_path.split_first() {
            let span = value.span();
            match member {
                PathMember::String { val, .. } => Ok(Value::record(
                    std::iter::once((val.clone(), Value::with_data_at_cell_path(path, value)?))
                        .collect(),
                    span,
                )),
                PathMember::Int { val, .. } => {
                    if *val == 0usize {
                        Ok(Value::list(
                            vec![Value::with_data_at_cell_path(path, value)?],
                            span,
                        ))
                    } else {
                        Err(ShellError::InsertAfterNextFreeIndex {
                            available_idx: 0,
                            span,
                        })
                    }
                }
            }
        } else {
            Ok(value)
        }
    }

    /// Visits all values contained within the value (including this value) with a mutable reference
    /// given to the closure.
    ///
    /// If the closure returns `Err`, the traversal will stop.
    ///
    /// Captures of closure values are currently visited, as they are values owned by the closure.
    pub fn recurse_mut<E>(
        &mut self,
        f: &mut impl FnMut(&mut Value) -> Result<(), E>,
    ) -> Result<(), E> {
        // Visit this value
        f(self)?;
        // Check for contained values
        match self {
            Value::Record { val, .. } => val
                .to_mut()
                .iter_mut()
                .try_for_each(|(_, rec_value)| rec_value.recurse_mut(f)),
            Value::List { vals, .. } => vals
                .iter_mut()
                .try_for_each(|list_value| list_value.recurse_mut(f)),
            // Closure captures are visited. Maybe these don't have to be if they are changed to
            // more opaque references.
            Value::Closure { val, .. } => val
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
            Value::Custom { .. } => Ok(()),
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

    pub fn filesize(val: impl Into<Filesize>, span: Span) -> Value {
        Value::Filesize {
            val: val.into(),
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
            val: val.into(),
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
            val: val.into(),
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
    pub fn test_filesize(val: impl Into<Filesize>) -> Value {
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

    /// Note: Only use this for test data, *not* live data,
    /// as it will point into unknown source when used in errors.
    ///
    /// Returns a `Vec` containing one of each value case (`Value::Int`, `Value::String`, etc.)
    /// except for `Value::Custom`.
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
            Value::test_list(Vec::new()),
            Value::test_closure(Closure {
                block_id: BlockId::new(0),
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

fn get_value_member<'a>(
    current: &'a Value,
    member: &PathMember,
) -> Result<ControlFlow<Span, Cow<'a, Value>>, ShellError> {
    match member {
        PathMember::Int {
            val: count,
            span: origin_span,
            optional,
        } => {
            // Treat a numeric path member as `select <val>`
            match current {
                Value::List { vals, .. } => {
                    if *count < vals.len() {
                        Ok(ControlFlow::Continue(Cow::Borrowed(&vals[*count])))
                    } else if *optional {
                        Ok(ControlFlow::Break(*origin_span))
                        // short-circuit
                    } else if vals.is_empty() {
                        Err(ShellError::AccessEmptyContent { span: *origin_span })
                    } else {
                        Err(ShellError::AccessBeyondEnd {
                            max_idx: vals.len() - 1,
                            span: *origin_span,
                        })
                    }
                }
                Value::Binary { val, .. } => {
                    if let Some(item) = val.get(*count) {
                        Ok(ControlFlow::Continue(Cow::Owned(Value::int(
                            *item as i64,
                            *origin_span,
                        ))))
                    } else if *optional {
                        Ok(ControlFlow::Break(*origin_span))
                        // short-circuit
                    } else if val.is_empty() {
                        Err(ShellError::AccessEmptyContent { span: *origin_span })
                    } else {
                        Err(ShellError::AccessBeyondEnd {
                            max_idx: val.len() - 1,
                            span: *origin_span,
                        })
                    }
                }
                Value::Range { val, .. } => {
                    if let Some(item) = val
                        .into_range_iter(current.span(), Signals::empty())
                        .nth(*count)
                    {
                        Ok(ControlFlow::Continue(Cow::Owned(item)))
                    } else if *optional {
                        Ok(ControlFlow::Break(*origin_span))
                        // short-circuit
                    } else {
                        Err(ShellError::AccessBeyondEndOfStream {
                            span: *origin_span,
                        })
                    }
                }
                Value::Custom { val, .. } => {
                    match val.follow_path_int(current.span(), *count, *origin_span)
                    {
                        Ok(val) => Ok(ControlFlow::Continue(Cow::Owned(val))),
                        Err(err) => {
                            if *optional {
                                Ok(ControlFlow::Break(*origin_span))
                                // short-circuit
                            } else {
                                Err(err)
                            }
                        }
                    }
                }
                Value::Nothing { .. } if *optional => Ok(ControlFlow::Break(*origin_span)),
                // Records (and tables) are the only built-in which support column names,
                // so only use this message for them.
                Value::Record { .. } => Err(ShellError::TypeMismatch {
                    err_message:"Can't access record values with a row index. Try specifying a column name instead".into(),
                    span: *origin_span,
                }),
                Value::Error { error, .. } => Err(*error.clone()),
                x => Err(ShellError::IncompatiblePathAccess { type_name: format!("{}", x.get_type()), span: *origin_span }),
            }
        }
        PathMember::String {
            val: column_name,
            span: origin_span,
            optional,
            casing,
        } => {
            let span = current.span();
            match current {
                Value::Record { val, .. } => {
                    let found = val.cased(*casing).get(column_name);
                    if let Some(found) = found {
                        Ok(ControlFlow::Continue(Cow::Borrowed(found)))
                    } else if *optional {
                        Ok(ControlFlow::Break(*origin_span))
                        // short-circuit
                    } else if let Some(suggestion) = did_you_mean(val.columns(), column_name) {
                        Err(ShellError::DidYouMean {
                            suggestion,
                            span: *origin_span,
                        })
                    } else {
                        Err(ShellError::CantFindColumn {
                            col_name: column_name.clone(),
                            span: Some(*origin_span),
                            src_span: span,
                        })
                    }
                }
                // String access of Lists always means Table access.
                // Create a List which contains each matching value for contained
                // records in the source list.
                Value::List { vals, .. } => {
                    let list = vals
                        .iter()
                        .map(|val| {
                            let val_span = val.span();
                            match val {
                                Value::Record { val, .. } => {
                                    let found = val.cased(*casing).get(column_name);
                                    if let Some(found) = found {
                                        Ok(found.clone())
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
                                            span: Some(*origin_span),
                                            src_span: val_span,
                                        })
                                    }
                                }
                                Value::Nothing { .. } if *optional => {
                                    Ok(Value::nothing(*origin_span))
                                }
                                _ => Err(ShellError::CantFindColumn {
                                    col_name: column_name.clone(),
                                    span: Some(*origin_span),
                                    src_span: val_span,
                                }),
                            }
                        })
                        .collect::<Result<_, _>>()?;

                    Ok(ControlFlow::Continue(Cow::Owned(Value::list(list, span))))
                }
                Value::Custom { val, .. } => {
                    match val.follow_path_string(current.span(), column_name.clone(), *origin_span)
                    {
                        Ok(val) => Ok(ControlFlow::Continue(Cow::Owned(val))),
                        Err(err) => {
                            if *optional {
                                Ok(ControlFlow::Break(*origin_span))
                                // short-circuit
                            } else {
                                Err(err)
                            }
                        }
                    }
                }
                Value::Nothing { .. } if *optional => Ok(ControlFlow::Break(*origin_span)),
                Value::Error { error, .. } => Err(error.as_ref().clone()),
                x => Err(ShellError::IncompatiblePathAccess {
                    type_name: format!("{}", x.get_type()),
                    span: *origin_span,
                }),
            }
        }
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
                Value::String { .. } => Some(Ordering::Less),
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Int { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Float { val: rhs, .. } => compare_floats(*lhs as f64, *rhs),
                Value::String { .. } => Some(Ordering::Less),
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Float { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { val: rhs, .. } => compare_floats(*lhs, *rhs as f64),
                Value::Float { val: rhs, .. } => compare_floats(*lhs, *rhs),
                Value::String { .. } => Some(Ordering::Less),
                Value::Glob { .. } => Some(Ordering::Less),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::String { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Glob { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Glob { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Glob { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Filesize { .. } => Some(Ordering::Less),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Filesize { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Duration { .. } => Some(Ordering::Less),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Duration { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Date { .. } => Some(Ordering::Less),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Date { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Range { .. } => Some(Ordering::Less),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Range { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Record { .. } => Some(Ordering::Less),
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Record { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
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
                Value::List { .. } => Some(Ordering::Less),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::List { vals: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { vals: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Closure { .. } => Some(Ordering::Less),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Closure { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { val: rhs, .. } => lhs.block_id.partial_cmp(&rhs.block_id),
                Value::Error { .. } => Some(Ordering::Less),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Error { .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Equal),
                Value::Binary { .. } => Some(Ordering::Less),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Binary { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::CellPath { .. } => Some(Ordering::Less),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::CellPath { val: lhs, .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { .. } => Some(Ordering::Greater),
                Value::CellPath { val: rhs, .. } => lhs.partial_cmp(rhs),
                Value::Custom { .. } => Some(Ordering::Less),
                Value::Nothing { .. } => Some(Ordering::Less),
            },
            (Value::Custom { val: lhs, .. }, rhs) => lhs.partial_cmp(rhs),
            (Value::Nothing { .. }, rhs) => match rhs {
                Value::Bool { .. } => Some(Ordering::Greater),
                Value::Int { .. } => Some(Ordering::Greater),
                Value::Float { .. } => Some(Ordering::Greater),
                Value::String { .. } => Some(Ordering::Greater),
                Value::Glob { .. } => Some(Ordering::Greater),
                Value::Filesize { .. } => Some(Ordering::Greater),
                Value::Duration { .. } => Some(Ordering::Greater),
                Value::Date { .. } => Some(Ordering::Greater),
                Value::Range { .. } => Some(Ordering::Greater),
                Value::Record { .. } => Some(Ordering::Greater),
                Value::List { .. } => Some(Ordering::Greater),
                Value::Closure { .. } => Some(Ordering::Greater),
                Value::Error { .. } => Some(Ordering::Greater),
                Value::Binary { .. } => Some(Ordering::Greater),
                Value::CellPath { .. } => Some(Ordering::Greater),
                Value::Custom { .. } => Some(Ordering::Greater),
                Value::Nothing { .. } => Some(Ordering::Equal),
            },
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).is_some_and(Ordering::is_eq)
    }
}

impl Value {
    pub fn add(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add(*rhs) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "add operation overflowed".into(),
                        span,
                        help: Some("Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into()),
                     })
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
            (Value::Duration { val: lhs, .. }, Value::Date { val: rhs, .. }) => {
                if let Some(val) = rhs.checked_add_signed(chrono::Duration::nanoseconds(*lhs)) {
                    Ok(Value::date(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "addition operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Date { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_add_signed(chrono::Duration::nanoseconds(*rhs)) {
                    Ok(Value::date(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "addition operation overflowed".into(),
                        span,
                        help: None,
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
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = *lhs + *rhs {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "add operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Add), op, rhs)
            }
            _ => Err(operator_type_error(
                Operator::Math(Math::Add),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::String { .. }
                            | Value::Date { .. }
                            | Value::Duration { .. }
                            | Value::Filesize { .. },
                    )
                },
            )),
        }
    }

    pub fn sub(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_sub(*rhs) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "subtraction operation overflowed".into(),
                        span,
                        help: Some("Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into()),
                    })
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
                        help: None,
                    })
                }
            }
            (Value::Date { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                match lhs.checked_sub_signed(chrono::Duration::nanoseconds(*rhs)) {
                    Some(val) => Ok(Value::date(val, span)),
                    _ => Err(ShellError::OperatorOverflow {
                        msg: "subtraction operation overflowed".into(),
                        span,
                        help: None,
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
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = *lhs - *rhs {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "add operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Subtract), op, rhs)
            }
            _ => Err(operator_type_error(
                Operator::Math(Math::Subtract),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::Date { .. }
                            | Value::Duration { .. }
                            | Value::Filesize { .. },
                    )
                },
            )),
        }
    }

    pub fn mul(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_mul(*rhs) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "multiply operation overflowed".into(),
                        span,
                        help: Some("Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into()),
                    })
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
                if let Some(val) = *lhs * *rhs {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "multiply operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = *lhs * *rhs {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "multiply operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = *lhs * *rhs {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "multiply operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = *lhs * *rhs {
                    Ok(Value::filesize(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "multiply operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
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
            _ => Err(operator_type_error(
                Operator::Math(Math::Multiply),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::Duration { .. }
                            | Value::Filesize { .. },
                    )
                },
            )),
        }
    }

    pub fn div(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Ok(Value::float(*lhs as f64 / *rhs as f64, span))
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
                if *rhs == Filesize::ZERO {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Ok(Value::float(lhs.get() as f64 / rhs.get() as f64, span))
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.get().checked_div(*rhs) {
                    Ok(Value::filesize(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    if let Ok(val) = Filesize::try_from(lhs.get() as f64 / rhs) {
                        Ok(Value::filesize(val, span))
                    } else {
                        Err(ShellError::OperatorOverflow {
                            msg: "division operation overflowed".into(),
                            span,
                            help: None,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Ok(Value::float(*lhs as f64 / *rhs as f64, span))
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = lhs.checked_div(*rhs) {
                    Ok(Value::duration(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    let val = *lhs as f64 / rhs;
                    if i64::MIN as f64 <= val && val <= i64::MAX as f64 {
                        Ok(Value::duration(val as i64, span))
                    } else {
                        Err(ShellError::OperatorOverflow {
                            msg: "division operation overflowed".into(),
                            span,
                            help: None,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Divide), op, rhs)
            }
            _ => Err(operator_type_error(
                Operator::Math(Math::Divide),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::Duration { .. }
                            | Value::Filesize { .. },
                    )
                },
            )),
        }
    }

    pub fn floor_div(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        // Taken from the unstable `div_floor` function in the std library.
        fn checked_div_floor_i64(dividend: i64, divisor: i64) -> Option<i64> {
            let quotient = dividend.checked_div(divisor)?;
            let remainder = dividend.checked_rem(divisor)?;
            if (remainder > 0 && divisor < 0) || (remainder < 0 && divisor > 0) {
                // Note that `quotient - 1` cannot overflow, because:
                //     `quotient` would have to be `i64::MIN`
                //     => `divisor` would have to be `1`
                //     => `remainder` would have to be `0`
                // But `remainder == 0` is excluded from the check above.
                Some(quotient - 1)
            } else {
                Some(quotient)
            }
        }

        fn checked_div_floor_f64(dividend: f64, divisor: f64) -> Option<f64> {
            if divisor == 0.0 {
                None
            } else {
                Some((dividend / divisor).floor())
            }
        }

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_i64(*lhs, *rhs) {
                    Ok(Value::int(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_f64(*lhs as f64, *rhs) {
                    Ok(Value::float(val, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_f64(*lhs, *rhs as f64) {
                    Ok(Value::float(val, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_f64(*lhs, *rhs) {
                    Ok(Value::float(val, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_i64(lhs.get(), rhs.get()) {
                    Ok(Value::int(val, span))
                } else if *rhs == Filesize::ZERO {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_i64(lhs.get(), *rhs) {
                    Ok(Value::filesize(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_f64(lhs.get() as f64, *rhs) {
                    if let Ok(val) = Filesize::try_from(val) {
                        Ok(Value::filesize(val, span))
                    } else {
                        Err(ShellError::OperatorOverflow {
                            msg: "division operation overflowed".into(),
                            span,
                            help: None,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_i64(*lhs, *rhs) {
                    Ok(Value::int(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_i64(*lhs, *rhs) {
                    Ok(Value::duration(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_div_floor_f64(*lhs as f64, *rhs) {
                    if i64::MIN as f64 <= val && val <= i64::MAX as f64 {
                        Ok(Value::duration(val as i64, span))
                    } else {
                        Err(ShellError::OperatorOverflow {
                            msg: "division operation overflowed".into(),
                            span,
                            help: None,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::FloorDivide), op, rhs)
            }
            _ => Err(operator_type_error(
                Operator::Math(Math::FloorDivide),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::Duration { .. }
                            | Value::Filesize { .. },
                    )
                },
            )),
        }
    }

    pub fn modulo(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        // Based off the unstable `div_floor` function in the std library.
        fn checked_mod_i64(dividend: i64, divisor: i64) -> Option<i64> {
            let remainder = dividend.checked_rem(divisor)?;
            if (remainder > 0 && divisor < 0) || (remainder < 0 && divisor > 0) {
                // Note that `remainder + divisor` cannot overflow, because `remainder` and
                // `divisor` have opposite signs.
                Some(remainder + divisor)
            } else {
                Some(remainder)
            }
        }

        fn checked_mod_f64(dividend: f64, divisor: f64) -> Option<f64> {
            if divisor == 0.0 {
                None
            } else {
                let remainder = dividend % divisor;
                if (remainder > 0.0 && divisor < 0.0) || (remainder < 0.0 && divisor > 0.0) {
                    Some(remainder + divisor)
                } else {
                    Some(remainder)
                }
            }
        }

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_mod_i64(*lhs, *rhs) {
                    Ok(Value::int(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "modulo operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_mod_f64(*lhs as f64, *rhs) {
                    Ok(Value::float(val, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_mod_f64(*lhs, *rhs as f64) {
                    Ok(Value::float(val, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_mod_f64(*lhs, *rhs) {
                    Ok(Value::float(val, span))
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                if let Some(val) = checked_mod_i64(lhs.get(), rhs.get()) {
                    Ok(Value::filesize(val, span))
                } else if *rhs == Filesize::ZERO {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "modulo operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_mod_i64(lhs.get(), *rhs) {
                    Ok(Value::filesize(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "modulo operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Filesize { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_mod_f64(lhs.get() as f64, *rhs) {
                    if let Ok(val) = Filesize::try_from(val) {
                        Ok(Value::filesize(val, span))
                    } else {
                        Err(ShellError::OperatorOverflow {
                            msg: "modulo operation overflowed".into(),
                            span,
                            help: None,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                if let Some(val) = checked_mod_i64(*lhs, *rhs) {
                    Ok(Value::duration(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if let Some(val) = checked_mod_i64(*lhs, *rhs) {
                    Ok(Value::duration(val, span))
                } else if *rhs == 0 {
                    Err(ShellError::DivisionByZero { span: op })
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "division operation overflowed".into(),
                        span,
                        help: None,
                    })
                }
            }
            (Value::Duration { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if let Some(val) = checked_mod_f64(*lhs as f64, *rhs) {
                    if i64::MIN as f64 <= val && val <= i64::MAX as f64 {
                        Ok(Value::duration(val as i64, span))
                    } else {
                        Err(ShellError::OperatorOverflow {
                            msg: "division operation overflowed".into(),
                            span,
                            help: None,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero { span: op })
                }
            }
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Math(Math::Modulo), op, rhs)
            }
            _ => Err(operator_type_error(
                Operator::Math(Math::Modulo),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::Duration { .. }
                            | Value::Filesize { .. },
                    )
                },
            )),
        }
    }

    pub fn pow(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhsv, .. }) => {
                if *rhsv < 0 {
                    return Err(ShellError::IncorrectValue {
                        msg: "Negative exponent for integer power is unsupported; use floats instead.".into(),
                        val_span: rhs.span(),
                        call_span: op,
                    });
                }

                if let Some(val) = lhs.checked_pow(*rhsv as u32) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "pow operation overflowed".into(),
                        span,
                        help: Some("Consider using floating point values for increased range by promoting operand with 'into float'. Note: float has reduced precision!".into()),
                    })
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
            _ => Err(operator_type_error(
                Operator::Math(Math::Pow),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Int { .. } | Value::Float { .. }),
            )),
        }
    }

    pub fn concat(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::List { vals: lhs, .. }, Value::List { vals: rhs, .. }) => {
                Ok(Value::list([lhs.as_slice(), rhs.as_slice()].concat(), span))
            }
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                Ok(Value::string([lhs.as_str(), rhs.as_str()].join(""), span))
            }
            (Value::Binary { val: lhs, .. }, Value::Binary { val: rhs, .. }) => Ok(Value::binary(
                [lhs.as_slice(), rhs.as_slice()].concat(),
                span,
            )),
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(self.span(), Operator::Math(Math::Concatenate), op, rhs)
            }
            _ => {
                let help = if matches!(self, Value::List { .. })
                    || matches!(rhs, Value::List { .. })
                {
                    Some(
                        "if you meant to append a value to a list or a record to a table, use the `append` command or wrap the value in a list. For example: `$list ++ $value` should be `$list ++ [$value]` or `$list | append $value`.",
                    )
                } else {
                    None
                };
                let is_supported = |val: &Value| {
                    matches!(
                        val,
                        Value::List { .. }
                            | Value::String { .. }
                            | Value::Binary { .. }
                            | Value::Custom { .. }
                    )
                };
                Err(match (is_supported(self), is_supported(rhs)) {
                    (true, true) => ShellError::OperatorIncompatibleTypes {
                        op: Operator::Math(Math::Concatenate),
                        lhs: self.get_type(),
                        rhs: rhs.get_type(),
                        op_span: op,
                        lhs_span: self.span(),
                        rhs_span: rhs.span(),
                        help,
                    },
                    (true, false) => ShellError::OperatorUnsupportedType {
                        op: Operator::Math(Math::Concatenate),
                        unsupported: rhs.get_type(),
                        op_span: op,
                        unsupported_span: rhs.span(),
                        help,
                    },
                    (false, _) => ShellError::OperatorUnsupportedType {
                        op: Operator::Math(Math::Concatenate),
                        unsupported: self.get_type(),
                        op_span: op,
                        unsupported_span: self.span(),
                        help,
                    },
                })
            }
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

        if !type_compatible(self.get_type(), rhs.get_type()) {
            return Err(operator_type_error(
                Operator::Comparison(Comparison::LessThan),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::String { .. }
                            | Value::Filesize { .. }
                            | Value::Duration { .. }
                            | Value::Date { .. }
                            | Value::Bool { .. }
                            | Value::Nothing { .. }
                    )
                },
            ));
        }

        Ok(Value::bool(
            matches!(self.partial_cmp(rhs), Some(Ordering::Less)),
            span,
        ))
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

        if !type_compatible(self.get_type(), rhs.get_type()) {
            return Err(operator_type_error(
                Operator::Comparison(Comparison::LessThanOrEqual),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::String { .. }
                            | Value::Filesize { .. }
                            | Value::Duration { .. }
                            | Value::Date { .. }
                            | Value::Bool { .. }
                            | Value::Nothing { .. }
                    )
                },
            ));
        }

        Ok(Value::bool(
            matches!(
                self.partial_cmp(rhs),
                Some(Ordering::Less | Ordering::Equal)
            ),
            span,
        ))
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

        if !type_compatible(self.get_type(), rhs.get_type()) {
            return Err(operator_type_error(
                Operator::Comparison(Comparison::GreaterThan),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::String { .. }
                            | Value::Filesize { .. }
                            | Value::Duration { .. }
                            | Value::Date { .. }
                            | Value::Bool { .. }
                            | Value::Nothing { .. }
                    )
                },
            ));
        }

        Ok(Value::bool(
            matches!(self.partial_cmp(rhs), Some(Ordering::Greater)),
            span,
        ))
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

        if !type_compatible(self.get_type(), rhs.get_type()) {
            return Err(operator_type_error(
                Operator::Comparison(Comparison::GreaterThanOrEqual),
                op,
                self,
                rhs,
                |val| {
                    matches!(
                        val,
                        Value::Int { .. }
                            | Value::Float { .. }
                            | Value::String { .. }
                            | Value::Filesize { .. }
                            | Value::Duration { .. }
                            | Value::Date { .. }
                            | Value::Bool { .. }
                            | Value::Nothing { .. }
                    )
                },
            ));
        }

        Ok(Value::bool(
            matches!(
                self.partial_cmp(rhs),
                Some(Ordering::Greater | Ordering::Equal)
            ),
            span,
        ))
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

        Ok(Value::bool(
            matches!(self.partial_cmp(rhs), Some(Ordering::Equal)),
            span,
        ))
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

        Ok(Value::bool(
            !matches!(self.partial_cmp(rhs), Some(Ordering::Equal)),
            span,
        ))
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
            (lhs, rhs) => Err(
                if matches!(
                    rhs,
                    Value::List { .. }
                        | Value::Range { .. }
                        | Value::String { .. }
                        | Value::Record { .. }
                        | Value::Custom { .. }
                ) {
                    ShellError::OperatorIncompatibleTypes {
                        op: Operator::Comparison(Comparison::In),
                        lhs: lhs.get_type(),
                        rhs: rhs.get_type(),
                        op_span: op,
                        lhs_span: lhs.span(),
                        rhs_span: rhs.span(),
                        help: None,
                    }
                } else {
                    ShellError::OperatorUnsupportedType {
                        op: Operator::Comparison(Comparison::In),
                        unsupported: rhs.get_type(),
                        op_span: op,
                        unsupported_span: rhs.span(),
                        help: None,
                    }
                },
            ),
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
            (lhs, rhs) => Err(
                if matches!(
                    rhs,
                    Value::List { .. }
                        | Value::Range { .. }
                        | Value::String { .. }
                        | Value::Record { .. }
                        | Value::Custom { .. }
                ) {
                    ShellError::OperatorIncompatibleTypes {
                        op: Operator::Comparison(Comparison::NotIn),
                        lhs: lhs.get_type(),
                        rhs: rhs.get_type(),
                        op_span: op,
                        lhs_span: lhs.span(),
                        rhs_span: rhs.span(),
                        help: None,
                    }
                } else {
                    ShellError::OperatorUnsupportedType {
                        op: Operator::Comparison(Comparison::NotIn),
                        unsupported: rhs.get_type(),
                        op_span: op,
                        unsupported_span: rhs.span(),
                        help: None,
                    }
                },
            ),
        }
    }

    pub fn has(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        rhs.r#in(op, self, span)
    }

    pub fn not_has(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        rhs.r#not_in(op, self, span)
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
            _ => Err(operator_type_error(
                if invert {
                    Operator::Comparison(Comparison::NotRegexMatch)
                } else {
                    Operator::Comparison(Comparison::RegexMatch)
                },
                op,
                self,
                rhs,
                |val| matches!(val, Value::String { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Comparison(Comparison::StartsWith),
                op,
                self,
                rhs,
                |val| matches!(val, Value::String { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Comparison(Comparison::EndsWith),
                op,
                self,
                rhs,
                |val| matches!(val, Value::String { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Bits(Bits::BitOr),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Int { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Bits(Bits::BitXor),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Int { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Bits(Bits::BitAnd),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Int { .. }),
            )),
        }
    }

    pub fn bit_shl(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                // Currently we disallow negative operands like Rust's `Shl`
                // Cheap guarding with TryInto<u32>
                if let Some(val) = (*rhs).try_into().ok().and_then(|rhs| lhs.checked_shl(rhs)) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "right operand to bit-shl exceeds available bits in underlying data"
                            .into(),
                        span,
                        help: Some(format!("Limit operand to 0 <= rhs < {}", i64::BITS)),
                    })
                }
            }
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Bits(Bits::ShiftLeft), op, rhs)
            }
            _ => Err(operator_type_error(
                Operator::Bits(Bits::ShiftLeft),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Int { .. }),
            )),
        }
    }

    pub fn bit_shr(&self, op: Span, rhs: &Value, span: Span) -> Result<Value, ShellError> {
        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                // Currently we disallow negative operands like Rust's `Shr`
                // Cheap guarding with TryInto<u32>
                if let Some(val) = (*rhs).try_into().ok().and_then(|rhs| lhs.checked_shr(rhs)) {
                    Ok(Value::int(val, span))
                } else {
                    Err(ShellError::OperatorOverflow {
                        msg: "right operand to bit-shr exceeds available bits in underlying data"
                            .into(),
                        span,
                        help: Some(format!("Limit operand to 0 <= rhs < {}", i64::BITS)),
                    })
                }
            }
            (Value::Custom { val: lhs, .. }, rhs) => {
                lhs.operation(span, Operator::Bits(Bits::ShiftRight), op, rhs)
            }
            _ => Err(operator_type_error(
                Operator::Bits(Bits::ShiftRight),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Int { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Boolean(Boolean::Or),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Bool { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Boolean(Boolean::Xor),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Bool { .. }),
            )),
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
            _ => Err(operator_type_error(
                Operator::Boolean(Boolean::And),
                op,
                self,
                rhs,
                |val| matches!(val, Value::Bool { .. }),
            )),
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

fn operator_type_error(
    op: Operator,
    op_span: Span,
    lhs: &Value,
    rhs: &Value,
    is_supported: fn(&Value) -> bool,
) -> ShellError {
    let is_supported = |val| is_supported(val) || matches!(val, Value::Custom { .. });
    match (is_supported(lhs), is_supported(rhs)) {
        (true, true) => ShellError::OperatorIncompatibleTypes {
            op,
            lhs: lhs.get_type(),
            rhs: rhs.get_type(),
            op_span,
            lhs_span: lhs.span(),
            rhs_span: rhs.span(),
            help: None,
        },
        (true, false) => ShellError::OperatorUnsupportedType {
            op,
            unsupported: rhs.get_type(),
            op_span,
            unsupported_span: rhs.span(),
            help: None,
        },
        (false, _) => ShellError::OperatorUnsupportedType {
            op,
            unsupported: lhs.get_type(),
            op_span,
            unsupported_span: lhs.span(),
            help: None,
        },
    }
}

pub fn human_time_from_now(val: &DateTime<FixedOffset>) -> HumanTime {
    let now = Local::now().with_timezone(val.offset());
    let delta = *val - now;
    match delta.num_nanoseconds() {
        Some(num_nanoseconds) => {
            let delta_seconds = num_nanoseconds as f64 / 1_000_000_000.0;
            let delta_seconds_rounded = delta_seconds.round() as i64;
            HumanTime::from(Duration::seconds(delta_seconds_rounded))
        }
        None => {
            // Happens if the total number of nanoseconds exceeds what fits in an i64
            // Note: not using delta.num_days() because it results is wrong for years before ~936: a extra year is added
            let delta_years = val.year() - now.year();
            HumanTime::from(Duration::days(delta_years as i64 * 365))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Record, Value};
    use crate::record;

    mod at_cell_path {
        use crate::casing::Casing;

        use crate::{IntoValue, Span};

        use super::super::PathMember;
        use super::*;

        #[test]
        fn test_record_with_data_at_cell_path() {
            let value_to_insert = Value::test_string("value");
            let span = Span::test_data();
            assert_eq!(
                Value::with_data_at_cell_path(
                    &[
                        PathMember::test_string("a".to_string(), false, Casing::Sensitive),
                        PathMember::test_string("b".to_string(), false, Casing::Sensitive),
                        PathMember::test_string("c".to_string(), false, Casing::Sensitive),
                        PathMember::test_string("d".to_string(), false, Casing::Sensitive),
                    ],
                    value_to_insert,
                ),
                // {a:{b:c{d:"value"}}}
                Ok(record!(
                    "a" => record!(
                        "b" => record!(
                            "c" => record!(
                                "d" => Value::test_string("value")
                            ).into_value(span)
                        ).into_value(span)
                    ).into_value(span)
                )
                .into_value(span))
            );
        }

        #[test]
        fn test_lists_with_data_at_cell_path() {
            let value_to_insert = Value::test_string("value");
            assert_eq!(
                Value::with_data_at_cell_path(
                    &[
                        PathMember::test_int(0, false),
                        PathMember::test_int(0, false),
                        PathMember::test_int(0, false),
                        PathMember::test_int(0, false),
                    ],
                    value_to_insert.clone(),
                ),
                // [[[[["value"]]]]]
                Ok(Value::test_list(vec![Value::test_list(vec![
                    Value::test_list(vec![Value::test_list(vec![value_to_insert])])
                ])]))
            );
        }
        #[test]
        fn test_mixed_with_data_at_cell_path() {
            let value_to_insert = Value::test_string("value");
            let span = Span::test_data();
            assert_eq!(
                Value::with_data_at_cell_path(
                    &[
                        PathMember::test_string("a".to_string(), false, Casing::Sensitive),
                        PathMember::test_int(0, false),
                        PathMember::test_string("b".to_string(), false, Casing::Sensitive),
                        PathMember::test_int(0, false),
                        PathMember::test_string("c".to_string(), false, Casing::Sensitive),
                        PathMember::test_int(0, false),
                        PathMember::test_string("d".to_string(), false, Casing::Sensitive),
                        PathMember::test_int(0, false),
                    ],
                    value_to_insert.clone(),
                ),
                // [{a:[{b:[{c:[{d:["value"]}]}]}]]}
                Ok(record!(
                    "a" => Value::test_list(vec![record!(
                        "b" => Value::test_list(vec![record!(
                            "c" => Value::test_list(vec![record!(
                                "d" => Value::test_list(vec![value_to_insert])
                            ).into_value(span)])
                        ).into_value(span)])
                    ).into_value(span)])
                )
                .into_value(span))
            );
        }

        #[test]
        fn test_nested_upsert_data_at_cell_path() {
            let span = Span::test_data();
            let mut base_value = record!(
                "a" => Value::test_list(vec![])
            )
            .into_value(span);

            let value_to_insert = Value::test_string("value");
            let res = base_value.upsert_data_at_cell_path(
                &[
                    PathMember::test_string("a".to_string(), false, Casing::Sensitive),
                    PathMember::test_int(0, false),
                    PathMember::test_string("b".to_string(), false, Casing::Sensitive),
                    PathMember::test_int(0, false),
                ],
                value_to_insert.clone(),
            );
            assert_eq!(res, Ok(()));
            assert_eq!(
                base_value,
                // {a:[{b:["value"]}]}
                record!(
                    "a" => Value::test_list(vec![
                        record!(
                            "b" => Value::test_list(vec![value_to_insert])
                        )
                        .into_value(span)
                    ])
                )
                .into_value(span)
            );
        }

        #[test]
        fn test_nested_insert_data_at_cell_path() {
            let span = Span::test_data();
            let mut base_value = record!(
                "a" => Value::test_list(vec![])
            )
            .into_value(span);

            let value_to_insert = Value::test_string("value");
            let res = base_value.insert_data_at_cell_path(
                &[
                    PathMember::test_string("a".to_string(), false, Casing::Sensitive),
                    PathMember::test_int(0, false),
                    PathMember::test_string("b".to_string(), false, Casing::Sensitive),
                    PathMember::test_int(0, false),
                ],
                value_to_insert.clone(),
                span,
            );
            assert_eq!(res, Ok(()));
            assert_eq!(
                base_value,
                // {a:[{b:["value"]}]}
                record!(
                    "a" => Value::test_list(vec![
                        record!(
                            "b" => Value::test_list(vec![value_to_insert])
                        )
                        .into_value(span)
                    ])
                )
                .into_value(span)
            );
        }
    }

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

    mod is_subtype {
        use crate::Type;

        use super::*;

        fn assert_subtype_equivalent(value: &Value, ty: &Type) {
            assert_eq!(value.is_subtype_of(ty), value.get_type().is_subtype_of(ty));
        }

        #[test]
        fn test_list() {
            let ty_int_list = Type::list(Type::Int);
            let ty_str_list = Type::list(Type::String);
            let ty_any_list = Type::list(Type::Any);
            let ty_list_list_int = Type::list(Type::list(Type::Int));

            let list = Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
            ]);

            assert_subtype_equivalent(&list, &ty_int_list);
            assert_subtype_equivalent(&list, &ty_str_list);
            assert_subtype_equivalent(&list, &ty_any_list);

            let list = Value::test_list(vec![
                Value::test_int(1),
                Value::test_string("hi"),
                Value::test_int(3),
            ]);

            assert_subtype_equivalent(&list, &ty_int_list);
            assert_subtype_equivalent(&list, &ty_str_list);
            assert_subtype_equivalent(&list, &ty_any_list);

            let list = Value::test_list(vec![Value::test_list(vec![Value::test_int(1)])]);

            assert_subtype_equivalent(&list, &ty_list_list_int);

            // The type of an empty lists is a subtype of any list or table type
            let ty_table = Type::Table(Box::new([
                ("a".into(), Type::Int),
                ("b".into(), Type::Int),
                ("c".into(), Type::Int),
            ]));
            let empty = Value::test_list(vec![]);

            assert_subtype_equivalent(&empty, &ty_any_list);
            assert!(empty.is_subtype_of(&ty_int_list));
            assert!(empty.is_subtype_of(&ty_table));
        }

        #[test]
        fn test_record() {
            let ty_abc = Type::Record(Box::new([
                ("a".into(), Type::Int),
                ("b".into(), Type::Int),
                ("c".into(), Type::Int),
            ]));
            let ty_ab = Type::Record(Box::new([("a".into(), Type::Int), ("b".into(), Type::Int)]));
            let ty_inner = Type::Record(Box::new([("inner".into(), ty_abc.clone())]));

            let record_abc = Value::test_record(record! {
                "a" => Value::test_int(1),
                "b" => Value::test_int(2),
                "c" => Value::test_int(3),
            });
            let record_ab = Value::test_record(record! {
                "a" => Value::test_int(1),
                "b" => Value::test_int(2),
            });

            assert_subtype_equivalent(&record_abc, &ty_abc);
            assert_subtype_equivalent(&record_abc, &ty_ab);
            assert_subtype_equivalent(&record_ab, &ty_abc);
            assert_subtype_equivalent(&record_ab, &ty_ab);

            let record_inner = Value::test_record(record! {
                "inner" => record_abc
            });
            assert_subtype_equivalent(&record_inner, &ty_inner);
        }

        #[test]
        fn test_table() {
            let ty_abc = Type::Table(Box::new([
                ("a".into(), Type::Int),
                ("b".into(), Type::Int),
                ("c".into(), Type::Int),
            ]));
            let ty_ab = Type::Table(Box::new([("a".into(), Type::Int), ("b".into(), Type::Int)]));
            let ty_list_any = Type::list(Type::Any);

            let record_abc = Value::test_record(record! {
                "a" => Value::test_int(1),
                "b" => Value::test_int(2),
                "c" => Value::test_int(3),
            });
            let record_ab = Value::test_record(record! {
                "a" => Value::test_int(1),
                "b" => Value::test_int(2),
            });

            let table_abc = Value::test_list(vec![record_abc.clone(), record_abc.clone()]);
            let table_ab = Value::test_list(vec![record_ab.clone(), record_ab.clone()]);

            assert_subtype_equivalent(&table_abc, &ty_abc);
            assert_subtype_equivalent(&table_abc, &ty_ab);
            assert_subtype_equivalent(&table_ab, &ty_abc);
            assert_subtype_equivalent(&table_ab, &ty_ab);
            assert_subtype_equivalent(&table_abc, &ty_list_any);

            let table_mixed = Value::test_list(vec![record_abc.clone(), record_ab.clone()]);
            assert_subtype_equivalent(&table_mixed, &ty_abc);
            assert!(table_mixed.is_subtype_of(&ty_ab));

            let ty_a = Type::Table(Box::new([("a".into(), Type::Any)]));
            let table_mixed_types = Value::test_list(vec![
                Value::test_record(record! {
                    "a" => Value::test_int(1),
                }),
                Value::test_record(record! {
                    "a" => Value::test_string("a"),
                }),
            ]);
            assert!(table_mixed_types.is_subtype_of(&ty_a));
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

    #[test]
    fn test_env_as_bool() {
        // explicit false values
        assert_eq!(Value::test_bool(false).coerce_bool(), Ok(false));
        assert_eq!(Value::test_int(0).coerce_bool(), Ok(false));
        assert_eq!(Value::test_float(0.0).coerce_bool(), Ok(false));
        assert_eq!(Value::test_string("").coerce_bool(), Ok(false));
        assert_eq!(Value::test_string("0").coerce_bool(), Ok(false));
        assert_eq!(Value::test_nothing().coerce_bool(), Ok(false));

        // explicit true values
        assert_eq!(Value::test_bool(true).coerce_bool(), Ok(true));
        assert_eq!(Value::test_int(1).coerce_bool(), Ok(true));
        assert_eq!(Value::test_float(1.0).coerce_bool(), Ok(true));
        assert_eq!(Value::test_string("1").coerce_bool(), Ok(true));

        // implicit true values
        assert_eq!(Value::test_int(42).coerce_bool(), Ok(true));
        assert_eq!(Value::test_float(0.5).coerce_bool(), Ok(true));
        assert_eq!(Value::test_string("not zero").coerce_bool(), Ok(true));

        // complex values returning None
        assert!(Value::test_record(Record::default()).coerce_bool().is_err());
        assert!(
            Value::test_list(vec![Value::test_int(1)])
                .coerce_bool()
                .is_err()
        );
        assert!(
            Value::test_date(
                chrono::DateTime::parse_from_rfc3339("2024-01-01T12:00:00+00:00").unwrap(),
            )
            .coerce_bool()
            .is_err()
        );
        assert!(Value::test_glob("*.rs").coerce_bool().is_err());
        assert!(Value::test_binary(vec![1, 2, 3]).coerce_bool().is_err());
        assert!(Value::test_duration(3600).coerce_bool().is_err());
    }
}
