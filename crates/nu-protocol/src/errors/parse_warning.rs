use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseWarning {
    #[error("Deprecated: {old_command}")]
    #[diagnostic(help("for more info: {url}"))]
    DeprecatedWarning {
        old_command: String,
        new_suggestion: String,
        #[label("`{old_command}` is deprecated and will be removed in 0.94. Please {new_suggestion} instead")]
        span: Span,
        url: String,
    },
}

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecatedWarning { span, .. } => *span,
        }
    }
}
