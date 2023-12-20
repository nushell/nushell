use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseWarning {
    #[error("Deprecated: {0}")]
    DeprecatedWarning(
        String,
        String,
        #[label = "`{0}` is deprecated and will be removed in 0.90. Please use `{1}` instead, more info: https://www.nushell.sh/book/custom_commands.html"]
        Span,
    ),
}

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecatedWarning(_, _, s) => *s,
        }
    }
}
