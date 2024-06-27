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
    #[error("internal compiler error: {msg}")]
    #[diagnostic(
        help("this is a bug, please report it at https://github.com/nushell/nushell/issues/new along with the code you were compiling if able")
    )]
    IrCompileError {
        msg: String,
        #[label = "while compiling this code"]
        span: Span,
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
