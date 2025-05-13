use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseWarning {
    #[error("Deprecated: {old_command}")]
    #[diagnostic(help("for more info see {url}"))]
    DeprecatedWarning {
        old_command: String,
        new_suggestion: String,
        #[label("`{old_command}` is deprecated and will be removed in a future release. Please {new_suggestion} instead.")]
        span: Span,
        url: String,
    },

    #[error("Found $in at the start of a command.")]
    #[diagnostic(help("Using $in at the start of a command collects the pipeline input.\nIf you did mean to collect the pipeline input, replace this with the `collect` command."))]
    UnnecessaryInVariable {
        #[label("try removing this")]
        span: Span,
    },
}

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecatedWarning { span, .. } => *span,
            ParseWarning::UnnecessaryInVariable { span, .. } => *span,
        }
    }
}
