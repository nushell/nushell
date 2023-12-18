use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseWarning {
    #[error("{0} is deprecated, use {1} instead")]
    DeprecatedWarning(String, String, #[label = "deprecated"] Span),
}
impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecatedWarning(_, _, s) => *s,
        }
    }
}
