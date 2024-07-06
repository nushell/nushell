use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{record, Config, Span, Value};

use super::helper::ReconstructVal;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub enum CompletionAlgorithm {
    #[default]
    Prefix,
    Fuzzy,
}

impl FromStr for CompletionAlgorithm {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "prefix" => Ok(Self::Prefix),
            "fuzzy" => Ok(Self::Fuzzy),
            _ => Err("expected either 'prefix' or 'fuzzy'"),
        }
    }
}

impl ReconstructVal for CompletionAlgorithm {
    fn reconstruct_value(&self, span: Span) -> Value {
        let str = match self {
            CompletionAlgorithm::Prefix => "prefix",
            CompletionAlgorithm::Fuzzy => "fuzzy",
        };
        Value::string(str, span)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub enum CompletionSort {
    #[default]
    Default,
    Alpha,
}

impl FromStr for CompletionSort {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "default" => Ok(Self::Default),
            "alpha" => Ok(Self::Alpha),
            _ => Err("expected either 'default' or 'alpha'"),
        }
    }
}

impl ReconstructVal for CompletionSort {
    fn reconstruct_value(&self, span: Span) -> Value {
        let str = match self {
            Self::Default => "default",
            Self::Alpha => "alpha",
        };
        Value::string(str, span)
    }
}

pub(super) fn reconstruct_external_completer(config: &Config, span: Span) -> Value {
    if let Some(closure) = config.external_completer.as_ref() {
        Value::closure(closure.clone(), span)
    } else {
        Value::nothing(span)
    }
}

pub(super) fn reconstruct_external(config: &Config, span: Span) -> Value {
    Value::record(
        record! {
            "max_results" => Value::int(config.max_external_completion_results, span),
            "completer" => reconstruct_external_completer(config, span),
            "enable" => Value::bool(config.enable_external_completion, span),
        },
        span,
    )
}
