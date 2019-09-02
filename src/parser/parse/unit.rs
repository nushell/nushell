use crate::object::base::Value;
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Unit {
    B,
    KB,
    MB,
    GB,
    TB,
    PB,
}

impl Unit {
    pub fn as_str(&self) -> &str {
        match *self {
            Unit::B => "B",
            Unit::KB => "KB",
            Unit::MB => "MB",
            Unit::GB => "GB",
            Unit::TB => "TB",
            Unit::PB => "PB",
        }
    }

    pub(crate) fn compute(&self, size: &Number) -> Value {
        let size = size.clone();

        Value::number(match self {
            Unit::B => size,
            Unit::KB => size * 1024,
            Unit::MB => size * 1024 * 1024,
            Unit::GB => size * 1024 * 1024 * 1024,
            Unit::TB => size * 1024 * 1024 * 1024 * 1024,
            Unit::PB => size * 1024 * 1024 * 1024 * 1024 * 1024,
        })
    }
}

impl From<&str> for Unit {
    fn from(input: &str) -> Unit {
        Unit::from_str(input).unwrap()
    }
}

impl FromStr for Unit {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, <Self as std::str::FromStr>::Err> {
        match input {
            "B" | "b" => Ok(Unit::B),
            "KB" | "kb" | "Kb" | "K" | "k" => Ok(Unit::KB),
            "MB" | "mb" | "Mb" => Ok(Unit::MB),
            "GB" | "gb" | "Gb" => Ok(Unit::GB),
            "TB" | "tb" | "Tb" => Ok(Unit::TB),
            "PB" | "pb" | "Pb" => Ok(Unit::PB),
            _ => Err(()),
        }
    }
}
