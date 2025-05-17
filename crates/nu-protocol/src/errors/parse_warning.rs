use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseWarning {
    #[error("Command deprecated.")]
    #[diagnostic(code(nu::parser::deprecated))]
    DeprecatedWarning {
        old_command: String,
        #[label("{old_command} is deprecated and will be removed in a future release.")]
        span: Span,
    },
    #[error("Command deprecated.")]
    #[diagnostic(code(nu::parser::deprecated))]
    #[diagnostic(help("{help}"))]
    DeprecatedWarningWithMessage {
        old_command: String,
        #[label("{old_command} is deprecated and will be removed in a future release.")]
        span: Span,
        help: String,
    },
}

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecatedWarning { span, .. } => *span,
            ParseWarning::DeprecatedWarningWithMessage { span, .. } => *span,
        }
    }
}
