use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use thiserror::Error;

use super::ReportMode;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseWarning {
    #[error("{warning}")]
    #[diagnostic(code(nu::parser::deprecated))]
    DeprecationWarning {
        warning: String,
        #[label("{label}")]
        span: Span,
        label: String,
        report_mode: ReportMode,
        #[help]
        help: Option<String>,
    },
}

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecationWarning { span, .. } => *span,
        }
    }

    pub fn report_mode(&self) -> ReportMode {
        match self {
            ParseWarning::DeprecationWarning { report_mode, .. } => *report_mode,
        }
    }
}

// To keep track of reported warnings
impl Hash for ParseWarning {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ParseWarning::DeprecationWarning { warning, label, .. } => {
                warning.hash(state);
                label.hash(state);
            }
        }
    }
}
