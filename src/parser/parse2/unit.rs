use serde_derive::{Deserialize, Serialize};
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
    pub fn print(&self) -> String {
        self.as_str().to_string()
    }

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
            "B" => Ok(Unit::B),
            "KB" => Ok(Unit::KB),
            "MB" => Ok(Unit::MB),
            "GB" => Ok(Unit::GB),
            "TB" => Ok(Unit::TB),
            "PB" => Ok(Unit::PB),
            _ => Err(()),
        }
    }
}
