use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use thiserror::Error;

use super::ReportMode;

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
pub enum ParseWarning {
    #[error("{dep_type} deprecated.")]
    #[diagnostic(code(nu::parser::deprecated))]
    DeprecationWarning {
        dep_type: String,
        #[label("{label}")]
        span: Span,
        label: String,
        report_mode: ReportMode,
    },
    #[error("{dep_type} deprecated.")]
    #[diagnostic(code(nu::parser::deprecated))]
    #[diagnostic(help("{help}"))]
    DeprecationWarningWithHelp {
        dep_type: String,
        #[label("{label}")]
        span: Span,
        label: String,
        report_mode: ReportMode,
        help: String,
    },
}

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::DeprecationWarning { span, .. } => *span,
            ParseWarning::DeprecationWarningWithHelp { span, .. } => *span,
        }
    }

    pub fn report_mode(&self) -> ReportMode {
        match self {
            ParseWarning::DeprecationWarning { report_mode, .. } => *report_mode,
            ParseWarning::DeprecationWarningWithHelp { report_mode, .. } => *report_mode,
        }
    }
}

// To keep track of reported warnings
impl Hash for ParseWarning {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ParseWarning::DeprecationWarningWithHelp {
                dep_type, label, ..
            }
            | ParseWarning::DeprecationWarning {
                dep_type, label, ..
            } => {
                dep_type.hash(state);
                label.hash(state);
            }
        }
    }
}
