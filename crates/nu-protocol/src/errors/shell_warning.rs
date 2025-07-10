use crate::Span;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use thiserror::Error;

use crate::{ReportMode, Reportable};

#[derive(Clone, Debug, Error, Diagnostic, Serialize, Deserialize)]
#[diagnostic(severity(Warning))]
pub enum ShellWarning {
    #[error("{dep_type} deprecated.")]
    #[diagnostic(code(nu::parser::deprecated))]
    Deprecated {
        dep_type: String,
        label: String,
        #[label("{label}")]
        span: Span,
        #[help]
        help: Option<String>,
        report_mode: ReportMode,
    },
}

impl ShellWarning {
    pub fn span(&self) -> Span {
        match self {
            ShellWarning::Deprecated { span, .. } => *span,
        }
    }
}

impl Reportable for ShellWarning {
    fn report_mode(&self) -> ReportMode {
        match self {
            ShellWarning::Deprecated { report_mode, .. } => *report_mode,
        }
    }
}

// To keep track of reported warnings
impl Hash for ShellWarning {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ShellWarning::Deprecated {
                dep_type, label, ..
            } => {
                dep_type.hash(state);
                label.hash(state);
            }
        }
    }
}
