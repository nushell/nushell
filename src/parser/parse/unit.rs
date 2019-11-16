use crate::data::base::Value;
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Unit {
    // Filesize units
    Byte,
    Kilobyte,
    Megabyte,
    Gigabyte,
    Terabyte,
    Petabyte,

    // Duration units
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

fn convert_number_to_u64(number: &Number) -> u64 {
    match number {
        Number::Int(big_int) => big_int.to_u64().unwrap(),
        Number::Decimal(big_decimal) => big_decimal.to_u64().unwrap(),
    }
}

impl FormatDebug for Spanned<Unit> {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        write!(f, "{}", self.span.slice(source))
    }
}

impl Unit {
    pub fn as_str(&self) -> &str {
        match *self {
            Unit::Byte => "B",
            Unit::Kilobyte => "KB",
            Unit::Megabyte => "MB",
            Unit::Gigabyte => "GB",
            Unit::Terabyte => "TB",
            Unit::Petabyte => "PB",
            Unit::Second => "s",
            Unit::Minute => "m",
            Unit::Hour => "h",
            Unit::Day => "d",
            Unit::Week => "w",
            Unit::Month => "M",
            Unit::Year => "y",
        }
    }

    pub(crate) fn compute(&self, size: &Number) -> Value {
        let size = size.clone();

        match self {
            Unit::Byte => Value::number(size),
            Unit::Kilobyte => Value::number(size * 1024),
            Unit::Megabyte => Value::number(size * 1024 * 1024),
            Unit::Gigabyte => Value::number(size * 1024 * 1024 * 1024),
            Unit::Terabyte => Value::number(size * 1024 * 1024 * 1024 * 1024),
            Unit::Petabyte => Value::number(size * 1024 * 1024 * 1024 * 1024 * 1024),
            Unit::Second => Value::system_date(
                SystemTime::now() - Duration::from_secs(convert_number_to_u64(&size)),
            ),
            Unit::Minute => Value::system_date(
                SystemTime::now() - Duration::from_secs(60 * convert_number_to_u64(&size)),
            ),
            Unit::Hour => Value::system_date(
                SystemTime::now() - Duration::from_secs(60 * 60 * convert_number_to_u64(&size)),
            ),
            Unit::Day => Value::system_date(
                SystemTime::now()
                    - Duration::from_secs(24 * 60 * 60 * convert_number_to_u64(&size)),
            ),
            Unit::Week => Value::system_date(
                SystemTime::now()
                    - Duration::from_secs(7 * 24 * 60 * 60 * convert_number_to_u64(&size)),
            ),
            Unit::Month => Value::system_date(
                SystemTime::now()
                    - Duration::from_secs(30 * 24 * 60 * 60 * convert_number_to_u64(&size)),
            ),
            Unit::Year => Value::system_date(
                SystemTime::now()
                    - Duration::from_secs(365 * 24 * 60 * 60 * convert_number_to_u64(&size)),
            ),
        }
    }
}

impl FromStr for Unit {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        match input {
            "B" | "b" => Ok(Unit::Byte),
            "KB" | "kb" | "Kb" | "K" | "k" => Ok(Unit::Kilobyte),
            "MB" | "mb" | "Mb" => Ok(Unit::Megabyte),
            "GB" | "gb" | "Gb" => Ok(Unit::Gigabyte),
            "TB" | "tb" | "Tb" => Ok(Unit::Terabyte),
            "PB" | "pb" | "Pb" => Ok(Unit::Petabyte),
            "s" => Ok(Unit::Second),
            "m" => Ok(Unit::Minute),
            "h" => Ok(Unit::Hour),
            "d" => Ok(Unit::Day),
            "w" => Ok(Unit::Week),
            "M" => Ok(Unit::Month),
            "y" => Ok(Unit::Year),
            _ => Err(()),
        }
    }
}
