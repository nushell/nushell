use crate::type_name::ShellTypeName;
use crate::value::column_path::ColumnPath;
use crate::value::range::{Range, RangeInclusion};
use crate::value::{serde_bigdecimal, serde_bigint};
use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset};
use chrono_humanize::HumanTime;
use nu_errors::{ExpectedRange, ShellError};
use nu_source::{PrettyDebug, Span, SpannedItem};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::cast::{FromPrimitive, ToPrimitive};
use num_traits::identities::Zero;
use num_traits::sign::Signed;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const NANOS_PER_SEC: u32 = 1_000_000_000;

/// The most fundamental of structured values in Nu are the Primitive values. These values represent types like integers, strings, booleans, dates, etc
/// that are then used as the building blocks of more complex structures.
///
/// Primitives also include marker values BeginningOfStream and EndOfStream which denote a change of condition in the stream
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum Primitive {
    /// An empty value
    Nothing,
    /// A common integer
    Int(i64),
    /// A "big int", an integer with arbitrarily large size (aka not limited to 64-bit)
    #[serde(with = "serde_bigint")]
    BigInt(BigInt),
    /// A "big decimal", an decimal number with arbitrarily large size (aka not limited to 64-bit)
    #[serde(with = "serde_bigdecimal")]
    Decimal(BigDecimal),
    /// A count in the number of bytes, used as a filesize
    Filesize(u64),
    /// A string value
    String(String),
    /// A path to travel to reach a value in a table
    ColumnPath(ColumnPath),
    /// A glob pattern, eg foo*
    GlobPattern(String),
    /// A boolean value
    Boolean(bool),
    /// A date value
    Date(DateTime<FixedOffset>),
    /// A count in the number of nanoseconds
    #[serde(with = "serde_bigint")]
    Duration(BigInt),
    /// A range of values
    Range(Box<Range>),
    /// A file path
    FilePath(PathBuf),
    /// A vector of raw binary data
    #[serde(with = "serde_bytes")]
    Binary(Vec<u8>),

    /// Beginning of stream marker, a pseudo-value not intended for tables
    BeginningOfStream,
    /// End of stream marker, a pseudo-value not intended for tables
    EndOfStream,
}

impl Primitive {
    /// Converts a primitive value to a char, if possible. Uses a span to build an error if the conversion isn't possible.
    pub fn as_char(&self, span: Span) -> Result<char, ShellError> {
        match self {
            Primitive::String(s) => {
                if s.len() > 1 {
                    return Err(ShellError::type_error(
                        "char",
                        self.type_name().spanned(span),
                    ));
                }
                s.chars()
                    .next()
                    .ok_or_else(|| ShellError::type_error("char", self.type_name().spanned(span)))
            }
            other => Err(ShellError::type_error(
                "char",
                other.type_name().spanned(span),
            )),
        }
    }

    /// Converts a primitive value to a u64, if possible. Uses a span to build an error if the conversion isn't possible.
    pub fn as_usize(&self, span: Span) -> Result<usize, ShellError> {
        match self {
            Primitive::Int(int) => int.to_usize().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U64,
                    &int.to_string().spanned(span),
                    "converting an integer into an unsigned 64-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_usize().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U64,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into an unsigned 64-bit integer",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    /// Converts a primitive value to a u64, if possible. Uses a span to build an error if the conversion isn't possible.
    pub fn as_u64(&self, span: Span) -> Result<u64, ShellError> {
        match self {
            Primitive::Int(int) => int.to_u64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U64,
                    &int.to_string().spanned(span),
                    "converting an integer into an unsigned 64-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_u64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U64,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into an unsigned 64-bit integer",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    /// Converts a primitive value to a f64, if possible. Uses a span to build an error if the conversion isn't possible.
    pub fn as_f64(&self, span: Span) -> Result<f64, ShellError> {
        match self {
            Primitive::Int(int) => int.to_f64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::F64,
                    &int.to_string().spanned(span),
                    "converting an integer into a 64-bit floating point",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_f64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::F64,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into a 64-bit floating point",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    /// Converts a primitive value to a i64, if possible. Uses a span to build an error if the conversion isn't possible.
    pub fn as_i64(&self, span: Span) -> Result<i64, ShellError> {
        match self {
            Primitive::Int(int) => int.to_i64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I64,
                    &int.to_string().spanned(span),
                    "converting an integer into a signed 64-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_i64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I64,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into a signed 64-bit integer",
                )
            }),
            Primitive::Duration(duration) => duration.to_i64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I64,
                    &duration.to_string().spanned(span),
                    "converting a duration into a signed 64-bit integer",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    /// Converts a primitive value to a u32, if possible. Uses a span to build an error if the conversion isn't possible.
    pub fn as_u32(&self, span: Span) -> Result<u32, ShellError> {
        match self {
            Primitive::Int(int) => int.to_u32().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U32,
                    &int.to_string().spanned(span),
                    "converting an integer into a unsigned 32-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_u32().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U32,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into a unsigned 32-bit integer",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    pub fn as_i32(&self, span: Span) -> Result<i32, ShellError> {
        match self {
            Primitive::Int(int) => int.to_i32().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I32,
                    &int.to_string().spanned(span),
                    "converting an integer into a signed 32-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_i32().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I32,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into a signed 32-bit integer",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    pub fn as_i16(&self, span: Span) -> Result<i16, ShellError> {
        match self {
            Primitive::Int(int) => int.to_i16().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I16,
                    &int.to_string().spanned(span),
                    "converting an integer into a signed 16-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_i16().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I16,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into a signed 16-bit integer",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    pub fn as_f32(&self, span: Span) -> Result<f32, ShellError> {
        match self {
            Primitive::Int(int) => int.to_f32().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::F32,
                    &int.to_string().spanned(span),
                    "converting an integer into a signed 32-bit float",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_f32().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::F32,
                    &decimal.to_string().spanned(span),
                    "converting a decimal into a signed 32-bit float",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    // FIXME: This is a bad name, but no other way to differentiate with our own Duration.
    pub fn into_chrono_duration(self, span: Span) -> Result<chrono::Duration, ShellError> {
        match self {
            Primitive::Duration(duration) => {
                // Divide into seconds because BigInt can be larger than i64
                let (secs, nanos) = duration.div_rem(
                    &BigInt::from_u32(NANOS_PER_SEC)
                        .expect("Internal error: conversion from u32 failed"),
                );
                let secs = match secs.to_i64() {
                    //The duration crate doesnt accept seconds bigger than i64::MAX / 1000
                    Some(secs) => match secs.checked_mul(1000) {
                        Some(_) => secs,
                        None => {
                            return Err(ShellError::labeled_error(
                                "Internal duration conversion overflow.",
                                "duration overflow",
                                span,
                            ))
                        }
                    },
                    None => {
                        return Err(ShellError::labeled_error(
                            "Internal duration conversion overflow.",
                            "duration overflow",
                            span,
                        ))
                    }
                };
                // This should never fail since NANOS_PER_SEC won't overflow
                let nanos = nanos.to_i64().expect("Unexpected i64 overflow");
                // This should also never fail since we are adding less than NANOS_PER_SEC.
                chrono::Duration::seconds(secs)
                    .checked_add(&chrono::Duration::nanoseconds(nanos))
                    .ok_or_else(|| ShellError::unexpected("Unexpected duration overflow"))
            }
            other => Err(ShellError::type_error(
                "duration",
                other.type_name().spanned(span),
            )),
        }
    }

    pub fn into_string(self, span: Span) -> Result<String, ShellError> {
        match self {
            Primitive::String(s) => Ok(s),
            other => Err(ShellError::type_error(
                "string",
                other.type_name().spanned(span),
            )),
        }
    }

    /// Returns true if the value is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Primitive::Nothing => true,
            Primitive::String(s) => s.is_empty(),
            _ => false,
        }
    }
}

impl From<bool> for Primitive {
    /// Helper to convert from boolean to a primitive
    fn from(b: bool) -> Primitive {
        Primitive::Boolean(b)
    }
}

impl From<&str> for Primitive {
    /// Helper to convert from string slices to a primitive
    fn from(s: &str) -> Primitive {
        Primitive::String(s.to_string())
    }
}

impl From<String> for Primitive {
    /// Helper to convert from Strings to a primitive
    fn from(s: String) -> Primitive {
        Primitive::String(s)
    }
}

impl From<BigDecimal> for Primitive {
    /// Helper to convert from decimals to a Primitive value
    fn from(decimal: BigDecimal) -> Primitive {
        Primitive::Decimal(decimal)
    }
}

impl From<BigInt> for Primitive {
    /// Helper to convert from integers to a Primitive value
    fn from(int: BigInt) -> Primitive {
        Primitive::BigInt(int)
    }
}

// Macro to define the From trait for native types to primitives
// The from trait requires a converter that will be applied to the
// native type.
macro_rules! from_native_to_primitive {
    ($native_type:ty, $primitive_type:expr, $converter: expr) => {
        // e.g. from u32 -> Primitive
        impl From<$native_type> for Primitive {
            fn from(value: $native_type) -> Primitive {
                if let Some(i) = $converter(value) {
                    $primitive_type(i)
                } else {
                    unreachable!("Internal error: protocol did not use compatible decimal")
                }
            }
        }
    };
}

from_native_to_primitive!(i8, Primitive::Int, i64::from_i8);
from_native_to_primitive!(i16, Primitive::Int, i64::from_i16);
from_native_to_primitive!(i32, Primitive::Int, i64::from_i32);
from_native_to_primitive!(i64, Primitive::Int, i64::from_i64);
from_native_to_primitive!(u8, Primitive::Int, i64::from_u8);
from_native_to_primitive!(u16, Primitive::Int, i64::from_u16);
from_native_to_primitive!(u32, Primitive::Int, i64::from_u32);
from_native_to_primitive!(u64, Primitive::BigInt, BigInt::from_u64);
from_native_to_primitive!(f32, Primitive::Decimal, BigDecimal::from_f32);
from_native_to_primitive!(f64, Primitive::Decimal, BigDecimal::from_f64);

impl From<chrono::Duration> for Primitive {
    fn from(duration: chrono::Duration) -> Primitive {
        // FIXME: This is a hack since chrono::Duration does not give access to its 'nanos' field.
        let secs: i64 = duration.num_seconds();
        // This will never fail.
        let nanos: u32 = duration
            .checked_sub(&chrono::Duration::seconds(secs))
            .expect("Unexpected overflow")
            .num_nanoseconds()
            .expect("Unexpected overflow") as u32;
        Primitive::Duration(
            BigInt::from_i64(secs * NANOS_PER_SEC as i64 + nanos as i64)
                .expect("Internal error: can't convert from i64"),
        )
    }
}

impl std::fmt::Display for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ShellTypeName for Primitive {
    /// Get the name of the type of a Primitive value
    fn type_name(&self) -> &'static str {
        match self {
            Primitive::Nothing => "nothing",
            Primitive::Int(_) => "integer",
            Primitive::BigInt(_) => "big integer",
            Primitive::Range(_) => "range",
            Primitive::Decimal(_) => "decimal",
            Primitive::Filesize(_) => "filesize(in bytes)",
            Primitive::String(_) => "string",
            Primitive::ColumnPath(_) => "column path",
            Primitive::GlobPattern(_) => "pattern",
            Primitive::Boolean(_) => "boolean",
            Primitive::Date(_) => "date",
            Primitive::Duration(_) => "duration",
            Primitive::FilePath(_) => "file path",
            Primitive::Binary(_) => "binary",
            Primitive::BeginningOfStream => "marker<beginning of stream>",
            Primitive::EndOfStream => "marker<end of stream>",
        }
    }
}

/// Format a Primitive value into a string
pub fn format_primitive(primitive: &Primitive, field_name: Option<&String>) -> String {
    match primitive {
        Primitive::Nothing => String::new(),
        Primitive::BeginningOfStream => String::new(),
        Primitive::EndOfStream => String::new(),
        Primitive::FilePath(p) => p.display().to_string(),
        Primitive::Filesize(num_bytes) => {
            if let Some(value) = num_bytes.to_u128() {
                let byte = byte_unit::Byte::from_bytes(value);

                if byte.get_bytes() == 0u128 {
                    return "—".to_string();
                }

                let byte = byte.get_appropriate_unit(false);

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => format!("{} B ", byte.get_value()),
                    _ => byte.format(1),
                }
            } else {
                format!("{} B", num_bytes)
            }
        }
        Primitive::Duration(duration) => format_duration(duration),
        Primitive::Int(i) => i.to_string(),
        Primitive::BigInt(i) => i.to_string(),
        Primitive::Decimal(decimal) => {
            // TODO: We should really pass the precision in here instead of hard coding it
            let decimal_string = decimal.to_string();
            let decimal_places: Vec<&str> = decimal_string.split('.').collect();
            if decimal_places.len() == 2 && decimal_places[1].len() > 4 {
                format!("{:.4}", decimal)
            } else {
                decimal.to_string()
            }
        }
        Primitive::Range(range) => format!(
            "{}..{}{}",
            format_primitive(&range.from.0.item, None),
            if range.to.1 == RangeInclusion::Exclusive {
                "<"
            } else {
                ""
            },
            format_primitive(&range.to.0.item, None)
        ),
        Primitive::GlobPattern(s) => s.to_string(),
        Primitive::String(s) => s.to_owned(),
        Primitive::ColumnPath(p) => {
            let mut members = p.iter();
            let mut f = String::new();

            f.push_str(
                &members
                    .next()
                    .expect("BUG: column path with zero members")
                    .display(),
            );

            for member in members {
                f.push('.');
                f.push_str(&member.display())
            }

            f
        }
        Primitive::Boolean(b) => match (b, field_name) {
            (true, None) => "true",
            (false, None) => "false",
            (true, Some(s)) if !s.is_empty() => s,
            (false, Some(s)) if !s.is_empty() => "",
            (true, Some(_)) => "true",
            (false, Some(_)) => "false",
        }
        .to_owned(),
        Primitive::Binary(_) => "<binary>".to_owned(),
        Primitive::Date(d) => format_date(d),
    }
}

/// Format a duration in nanoseconds into a string
pub fn format_duration(duration: &BigInt) -> String {
    let is_zero = duration.is_zero();
    // FIXME: This involves a lot of allocation, but it seems inevitable with BigInt.
    let big_int_1000 = BigInt::from(1000);
    let big_int_60 = BigInt::from(60);
    let big_int_24 = BigInt::from(24);
    // We only want the biggest subdivision to have the negative sign.
    let (sign, duration) = if duration.is_zero() || duration.is_positive() {
        (1, duration.clone())
    } else {
        (-1, -duration)
    };
    let (micros, nanos): (BigInt, BigInt) = duration.div_rem(&big_int_1000);
    let (millis, micros): (BigInt, BigInt) = micros.div_rem(&big_int_1000);
    let (secs, millis): (BigInt, BigInt) = millis.div_rem(&big_int_1000);
    let (mins, secs): (BigInt, BigInt) = secs.div_rem(&big_int_60);
    let (hours, mins): (BigInt, BigInt) = mins.div_rem(&big_int_60);
    let (days, hours): (BigInt, BigInt) = hours.div_rem(&big_int_24);

    let mut output_prep = vec![];

    if !days.is_zero() {
        output_prep.push(format!("{}day", days));
    }

    if !hours.is_zero() {
        output_prep.push(format!("{}hr", hours));
    }

    if !mins.is_zero() {
        output_prep.push(format!("{}min", mins));
    }
    // output 0sec for zero duration
    if is_zero || !secs.is_zero() {
        output_prep.push(format!("{}sec", secs));
    }

    if !millis.is_zero() {
        output_prep.push(format!("{}ms", millis));
    }

    if !micros.is_zero() {
        output_prep.push(format!("{}us", micros));
    }

    if !nanos.is_zero() {
        output_prep.push(format!("{}ns", nanos));
    }

    format!(
        "{}{}",
        if sign == -1 { "-" } else { "" },
        output_prep.join(" ")
    )
}

/// Format a date value into a humanized string (eg "1 week ago" instead of a formal date string)
pub fn format_date(d: &DateTime<FixedOffset>) -> String {
    HumanTime::from(*d).to_string()
}
