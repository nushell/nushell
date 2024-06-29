use crate::{CompileError, Span};
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
        #[label("`{old_command}` is deprecated and will be removed in a future release. Please {new_suggestion} instead")]
        span: Span,
        url: String,
    },

    /// An error occurred with the IR compiler.
    ///
    /// ## Resolution
    ///
    /// The IR compiler is in very early development, so code that can't be compiled is quite
    /// expected. If you think it should be working, please report it to us.
    #[error("IR compile error")]
    IrCompileError {
        #[label("failed to compile this code to IR instructions")]
        span: Span,
        #[related]
        errors: Vec<CompileError>,
    },
}

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecatedWarning { span, .. } => *span,
            ParseWarning::IrCompileError { span, .. } => *span,
        }
    }
}
