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
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum Primitive {
    Nothing,
    #[serde(with = "serde_bigint")]
    Int(BigInt),
    #[serde(with = "serde_bigdecimal")]
    Decimal(BigDecimal),
    Bytes(u64),
    String(String),
    Line(String),
    ColumnPath(ColumnPath),
    Pattern(String),
    Boolean(bool),
    Date(DateTime<Utc>),
    Duration(u64), // Duration in seconds
    Range(Box<Range>),
    Path(PathBuf),
    #[serde(with = "serde_bytes")]
    Binary(Vec<u8>),

    // Stream markers (used as bookend markers rather than actual values)
    BeginningOfStream,
    EndOfStream,
}

impl Primitive {
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
}

impl From<BigDecimal> for Primitive {
    fn from(decimal: BigDecimal) -> Primitive {
        Primitive::Decimal(decimal)
    }
}

impl From<f64> for Primitive {
    fn from(float: f64) -> Primitive {
        if let Some(f) = BigDecimal::from_f64(float) {
            Primitive::Decimal(f)
        } else {
            unreachable!("Internal error: protocol did not use f64-compatible decimal")
        }
    }
}

impl ShellTypeName for Primitive {
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

pub fn format_duration(sec: u64) -> String {
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

pub fn format_date(d: &DateTime<Utc>) -> String {
    let utc: DateTime<Utc> = Utc::now();

    let duration = utc.signed_duration_since(*d);

    if duration.num_weeks() >= 52 {
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
