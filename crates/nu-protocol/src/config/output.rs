use crate::{Config, Record, Span, Value};
use nu_derive_value::IntoValue;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

pub(super) fn reconstruct_datetime_format(config: &Config, span: Span) -> Value {
    let mut record = Record::new();
    if let Some(normal) = &config.datetime_normal_format {
        record.push("normal", Value::string(normal, span));
    }
    if let Some(table) = &config.datetime_table_format {
        record.push("table", Value::string(table, span));
    }
    Value::record(record, span)
}
