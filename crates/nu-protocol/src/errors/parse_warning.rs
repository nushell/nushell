use crate::{DeclId, Span};
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
        decl_id: DeclId,
    },
    #[error("Command deprecated.")]
    #[diagnostic(code(nu::parser::deprecated))]
    #[diagnostic(help("{help}"))]
    DeprecatedWarningWithMessage {
        old_command: String,
        #[label("{old_command} is deprecated and will be removed in a future release.")]
        span: Span,
        help: String,
        decl_id: DeclId,
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
