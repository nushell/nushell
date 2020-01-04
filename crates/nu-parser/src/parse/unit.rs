use crate::parse::parser::Number;
use nu_protocol::{Primitive, UntaggedValue};
use nu_source::{b, DebugDocBuilder, PrettyDebug};
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};

use std::str::FromStr;

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

impl PrettyDebug for Unit {
    fn pretty(&self) -> DebugDocBuilder {
        b::keyword(self.as_str())
    }
}

fn convert_number_to_u64(number: &Number) -> u64 {
    match number {
        Number::Int(big_int) => {
            if let Some(x) = big_int.to_u64() {
                x
            } else {
                unreachable!("Internal error: convert_number_to_u64 given incompatible number")
            }
        }
        Number::Decimal(big_decimal) => {
            if let Some(x) = big_decimal.to_u64() {
                x
            } else {
                unreachable!("Internal error: convert_number_to_u64 given incompatible number")
            }
        }
    }
}

impl Unit {
    pub fn as_str(self) -> &'static str {
        match self {
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

    pub fn compute(self, size: &Number) -> UntaggedValue {
        let size = size.clone();

        match self {
            Unit::Byte => number(size),
            Unit::Kilobyte => number(size * 1024),
            Unit::Megabyte => number(size * 1024 * 1024),
            Unit::Gigabyte => number(size * 1024 * 1024 * 1024),
            Unit::Terabyte => number(size * 1024 * 1024 * 1024 * 1024),
            Unit::Petabyte => number(size * 1024 * 1024 * 1024 * 1024 * 1024),
            Unit::Second => duration(convert_number_to_u64(&size)),
            Unit::Minute => duration(60 * convert_number_to_u64(&size)),
            Unit::Hour => duration(60 * 60 * convert_number_to_u64(&size)),
            Unit::Day => duration(24 * 60 * 60 * convert_number_to_u64(&size)),
            Unit::Week => duration(7 * 24 * 60 * 60 * convert_number_to_u64(&size)),
            Unit::Month => duration(30 * 24 * 60 * 60 * convert_number_to_u64(&size)),
            Unit::Year => duration(365 * 24 * 60 * 60 * convert_number_to_u64(&size)),
        }
    }
}

fn number(number: impl Into<Number>) -> UntaggedValue {
    let number = number.into();

    match number {
        Number::Int(int) => UntaggedValue::Primitive(Primitive::Int(int)),
        Number::Decimal(decimal) => UntaggedValue::Primitive(Primitive::Decimal(decimal)),
    }
}

pub fn duration(secs: u64) -> UntaggedValue {
    UntaggedValue::Primitive(Primitive::Duration(secs))
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
