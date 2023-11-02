use super::helper::ReconstructVal;
use crate::{Config, Record, Span, Value};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, Debug, Copy)]
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

impl ReconstructVal for ErrorStyle {
    fn reconstruct_value(&self, span: Span) -> Value {
        Value::string(
            match self {
                ErrorStyle::Fancy => "fancy",
                ErrorStyle::Plain => "plain",
            },
            span,
        )
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
