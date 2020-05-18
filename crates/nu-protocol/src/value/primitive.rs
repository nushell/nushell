use crate::type_name::ShellTypeName;
use crate::value::column_path::ColumnPath;
use crate::value::range::Range;
use crate::value::{serde_bigdecimal, serde_bigint};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use nu_errors::{ExpectedRange, ShellError};
use nu_source::{PrettyDebug, Span, SpannedItem};
use num_bigint::BigInt;
use num_traits::cast::{FromPrimitive, ToPrimitive};
use num_traits::identities::Zero;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The most fundamental of structured values in Nu are the Primitive values. These values represent types like integers, strings, booleans, dates, etc that are then used
/// as the buildig blocks to build up more complex structures.
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
    Bytes(u64),
    /// A string value
    String(String),
    /// A string value with an implied carriage return (or cr/lf) ending
    Line(String),
    /// A path to travel to reach a value in a table
    ColumnPath(ColumnPath),
    /// A glob pattern, eg foo*
    Pattern(String),
    /// A boolean value
    Boolean(bool),
    /// A date value, in UTC
    Date(DateTime<Utc>),
    /// A count in the number of seconds
    Duration(i64),
    /// A range of values
    Range(Box<Range>),
    /// A file path
    Path(PathBuf),
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
            Primitive::Int(int) => match int.to_u64() {
                None => Err(ShellError::range_error(
                    ExpectedRange::U64,
                    &format!("{}", int).spanned(span),
                    "converting an integer into a 64-bit integer",
                )),
                Some(num) => Ok(num),
            },
            other => Err(ShellError::type_error(
                "integer",
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

impl num_traits::Zero for Primitive {
    fn zero() -> Self {
        Primitive::Int(BigInt::zero())
    }

    fn is_zero(&self) -> bool {
        match self {
            Primitive::Int(int) => int.is_zero(),
            Primitive::Decimal(decimal) => decimal.is_zero(),
            Primitive::Bytes(size) => size.is_zero(),
            _ => false,
        }
    }
}

impl std::ops::Add for Primitive {
    type Output = Primitive;

    fn add(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Primitive::Int(left), Primitive::Int(right)) => Primitive::Int(left + right),
            (Primitive::Int(left), Primitive::Decimal(right)) => {
                Primitive::Decimal(BigDecimal::from(left) + right)
            }
            (Primitive::Decimal(left), Primitive::Decimal(right)) => {
                Primitive::Decimal(left + right)
            }
            (Primitive::Decimal(left), Primitive::Int(right)) => {
                Primitive::Decimal(left + BigDecimal::from(right))
            }
            (Primitive::Bytes(left), right) => match right {
                Primitive::Bytes(right) => Primitive::Bytes(left + right),
                Primitive::Int(right) => {
                    Primitive::Bytes(left + right.to_u64().unwrap_or_else(|| 0 as u64))
                }
                Primitive::Decimal(right) => {
                    Primitive::Bytes(left + right.to_u64().unwrap_or_else(|| 0 as u64))
                }
                _ => Primitive::Bytes(left),
            },
            (left, Primitive::Bytes(right)) => match left {
                Primitive::Bytes(left) => Primitive::Bytes(left + right),
                Primitive::Int(left) => {
                    Primitive::Bytes(left.to_u64().unwrap_or_else(|| 0 as u64) + right)
                }
                Primitive::Decimal(left) => {
                    Primitive::Bytes(left.to_u64().unwrap_or_else(|| 0 as u64) + right)
                }
                _ => Primitive::Bytes(right),
            },
            _ => Primitive::zero(),
        }
    }
}

impl std::ops::Mul for Primitive {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Primitive::Int(left), Primitive::Int(right)) => Primitive::Int(left * right),
            (Primitive::Int(left), Primitive::Decimal(right)) => {
                Primitive::Decimal(BigDecimal::from(left) * right)
            }
            (Primitive::Decimal(left), Primitive::Decimal(right)) => {
                Primitive::Decimal(left * right)
            }
            (Primitive::Decimal(left), Primitive::Int(right)) => {
                Primitive::Decimal(left * BigDecimal::from(right))
            }
            _ => unimplemented!("Internal error: can't multiply incompatible primitives."),
        }
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

impl ShellTypeName for Primitive {
    /// Get the name of the type of a Primitive value
    fn type_name(&self) -> &'static str {
        match self {
            Primitive::Nothing => "nothing",
            Primitive::Int(_) => "integer",
            Primitive::Range(_) => "range",
            Primitive::Decimal(_) => "decimal",
            Primitive::Bytes(_) => "bytes",
            Primitive::String(_) => "string",
            Primitive::Line(_) => "line",
            Primitive::ColumnPath(_) => "column path",
            Primitive::Pattern(_) => "pattern",
            Primitive::Boolean(_) => "boolean",
            Primitive::Date(_) => "date",
            Primitive::Duration(_) => "duration",
            Primitive::Path(_) => "file path",
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
        Primitive::Path(p) => format!("{}", p.display()),
        Primitive::Bytes(b) => {
            let byte = byte_unit::Byte::from_bytes(*b as u128);

            if byte.get_bytes() == 0u128 {
                return "—".to_string();
            }

            let byte = byte.get_appropriate_unit(false);

            match byte.get_unit() {
                byte_unit::ByteUnit::B => format!("{} B ", byte.get_value()),
                _ => byte.format(1),
            }
        }
        Primitive::Duration(sec) => format_duration(*sec),
        Primitive::Int(i) => i.to_string(),
        Primitive::Decimal(decimal) => format!("{:.4}", decimal),
        Primitive::Range(range) => format!(
            "{}..{}",
            format_primitive(&range.from.0.item, None),
            format_primitive(&range.to.0.item, None)
        ),
        Primitive::Pattern(s) => s.to_string(),
        Primitive::String(s) => s.to_owned(),
        Primitive::Line(s) => s.to_owned(),
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
                f.push_str(".");
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

/// Format a duration in seconds into a string
pub fn format_duration(sec: i64) -> String {
    let (minutes, seconds) = (sec / 60, sec % 60);
    let (hours, minutes) = (minutes / 60, minutes % 60);
    let (days, hours) = (hours / 24, hours % 24);

    match (days, hours, minutes, seconds) {
        (0, 0, 0, 1) => "1 sec".to_owned(),
        (0, 0, 0, s) => format!("{} secs", s),
        (0, 0, m, s) => format!("{}:{:02}", m, s),
        (0, h, m, s) => format!("{}:{:02}:{:02}", h, m, s),
        (d, h, m, s) => format!("{}:{:02}:{:02}:{:02}", d, h, m, s),
    }
}

#[allow(clippy::cognitive_complexity)]
/// Format a UTC date value into a humanized string (eg "1 week ago" instead of a formal date string)
pub fn format_date(d: &DateTime<Utc>) -> String {
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
