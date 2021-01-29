use crate::type_name::ShellTypeName;
use crate::value::column_path::ColumnPath;
use crate::value::range::{Range, RangeInclusion};
use crate::value::{serde_bigdecimal, serde_bigint};
use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset, Utc};
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
    /// A "big int", an integer with arbitrarily large size (aka not limited to 64-bit)
    #[serde(with = "serde_bigint")]
    Int(BigInt),
    /// A "big decimal", an decimal number with arbitrarily large size (aka not limited to 64-bit)
    #[serde(with = "serde_bigdecimal")]
    Decimal(BigDecimal),
    /// A count in the number of bytes, used as a filesize
    Filesize(BigInt),
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
    /// Converts a primitive value to a u64, if possible. Uses a span to build an error if the conversion isn't possible.
    pub fn as_u64(&self, span: Span) -> Result<u64, ShellError> {
        match self {
            Primitive::Int(int) => int.to_u64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U64,
                    &format!("{}", int).spanned(span),
                    "converting an integer into an unsigned 64-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_u64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::U64,
                    &format!("{}", decimal).spanned(span),
                    "converting a decimal into an unsigned 64-bit integer",
                )
            }),
            other => Err(ShellError::type_error(
                "number",
                other.type_name().spanned(span),
            )),
        }
    }

    pub fn as_i64(&self, span: Span) -> Result<i64, ShellError> {
        match self {
            Primitive::Int(int) => int.to_i64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I64,
                    &format!("{}", int).spanned(span),
                    "converting an integer into a signed 64-bit integer",
                )
            }),
            Primitive::Decimal(decimal) => decimal.to_i64().ok_or_else(|| {
                ShellError::range_error(
                    ExpectedRange::I64,
                    &format!("{}", decimal).spanned(span),
                    "converting a decimal into a signed 64-bit integer",
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
                let (secs, nanos) = duration.div_rem(&BigInt::from(NANOS_PER_SEC));
                let secs = match secs.to_i64() {
                    Some(secs) => secs,
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
        Primitive::Int(int)
    }
}

impl From<f64> for Primitive {
    /// Helper to convert from 64-bit float to a Primitive value
    fn from(float: f64) -> Primitive {
        if let Some(f) = BigDecimal::from_f64(float) {
            Primitive::Decimal(f)
        } else {
            unreachable!("Internal error: protocol did not use f64-compatible decimal")
        }
    }
}

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
        Primitive::Duration(BigInt::from(secs) * NANOS_PER_SEC + nanos)
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
        Primitive::FilePath(p) => format!("{}", p.display()),
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
        Primitive::Decimal(decimal) => {
            // TODO: We should really pass the precision in here instead of hard coding it
            let decimal_string = decimal.to_string();
            let decimal_places: Vec<&str> = decimal_string.split('.').collect();
            if decimal_places.len() == 2 && decimal_places[1].len() > 4 {
                format!("{:.4}", decimal)
            } else {
                format!("{}", decimal)
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
            (true, None) => "Yes",
            (false, None) => "No",
            (true, Some(s)) if !s.is_empty() => s,
            (false, Some(s)) if !s.is_empty() => "",
            (true, Some(_)) => "Yes",
            (false, Some(_)) => "No",
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

#[allow(clippy::cognitive_complexity)]
/// Format a date value into a humanized string (eg "1 week ago" instead of a formal date string)
pub fn format_date(d: &DateTime<FixedOffset>) -> String {
    let utc: DateTime<Utc> = Utc::now();

    let duration = utc.signed_duration_since(*d);

    if duration.num_seconds() < 0 {
        // Our duration is negative, so we need to speak about the future
        if -duration.num_weeks() >= 52 {
            let num_years = -duration.num_weeks() / 52;

            format!(
                "{} year{} from now",
                num_years,
                if num_years == 1 { "" } else { "s" }
            )
        } else if -duration.num_weeks() >= 4 {
            let num_months = -duration.num_weeks() / 4;

            format!(
                "{} month{} from now",
                num_months,
                if num_months == 1 { "" } else { "s" }
            )
        } else if -duration.num_weeks() >= 1 {
            let num_weeks = -duration.num_weeks();

            format!(
                "{} week{} from now",
                num_weeks,
                if num_weeks == 1 { "" } else { "s" }
            )
        } else if -duration.num_days() >= 1 {
            let num_days = -duration.num_days();

            format!(
                "{} day{} from now",
                num_days,
                if num_days == 1 { "" } else { "s" }
            )
        } else if -duration.num_hours() >= 1 {
            let num_hours = -duration.num_hours();

            format!(
                "{} hour{} from now",
                num_hours,
                if num_hours == 1 { "" } else { "s" }
            )
        } else if -duration.num_minutes() >= 1 {
            let num_minutes = -duration.num_minutes();

            format!(
                "{} min{} from now",
                num_minutes,
                if num_minutes == 1 { "" } else { "s" }
            )
        } else {
            let num_seconds = -duration.num_seconds();

            format!(
                "{} sec{} from now",
                num_seconds,
                if num_seconds == 1 { "" } else { "s" }
            )
        }
    } else if duration.num_weeks() >= 52 {
        let num_years = duration.num_weeks() / 52;

        format!(
            "{} year{} ago",
            num_years,
            if num_years == 1 { "" } else { "s" }
        )
    } else if duration.num_weeks() >= 4 {
        let num_months = duration.num_weeks() / 4;

        format!(
            "{} month{} ago",
            num_months,
            if num_months == 1 { "" } else { "s" }
        )
    } else if duration.num_weeks() >= 1 {
        let num_weeks = duration.num_weeks();

        format!(
            "{} week{} ago",
            num_weeks,
            if num_weeks == 1 { "" } else { "s" }
        )
    } else if duration.num_days() >= 1 {
        let num_days = duration.num_days();

        format!(
            "{} day{} ago",
            num_days,
            if num_days == 1 { "" } else { "s" }
        )
    } else if duration.num_hours() >= 1 {
        let num_hours = duration.num_hours();

        format!(
            "{} hour{} ago",
            num_hours,
            if num_hours == 1 { "" } else { "s" }
        )
    } else if duration.num_minutes() >= 1 {
        let num_minutes = duration.num_minutes();

        format!(
            "{} min{} ago",
            num_minutes,
            if num_minutes == 1 { "" } else { "s" }
        )
    } else {
        let num_seconds = duration.num_seconds();

        format!(
            "{} sec{} ago",
            num_seconds,
            if num_seconds == 1 { "" } else { "s" }
        )
    }
}
