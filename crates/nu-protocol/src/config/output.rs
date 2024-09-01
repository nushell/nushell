use super::prelude::*;
use crate as nu_protocol;

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorStyle {
    Plain,
    Fancy,
}

impl FromStr for ErrorStyle {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "fancy" => Ok(Self::Fancy),
            "plain" => Ok(Self::Plain),
            _ => Err("expected either 'fancy' or 'plain'"),
        }
    }
}
