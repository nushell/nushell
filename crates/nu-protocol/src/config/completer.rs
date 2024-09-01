use super::prelude::*;
use crate::Config;

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Default, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionSort {
    #[default]
    Smart,
    Alphabetical,
}

impl FromStr for CompletionSort {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "smart" => Ok(Self::Smart),
            "alphabetical" => Ok(Self::Alphabetical),
            _ => Err("expected either 'smart' or 'alphabetical'"),
        }
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
