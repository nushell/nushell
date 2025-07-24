use crate::Span;
use miette::Diagnostic;
use std::hash::Hash;
use thiserror::Error;

use crate::{ReportMode, Reportable};

#[derive(Clone, Debug, Error, Diagnostic)]
#[diagnostic(severity(Warning))]
pub enum ParseWarning {
    /// A parse-time deprectaion. Indicates that something will be removed in a future release.
    ///
    /// Use [`ShellWarning::Deprecated`] if this is a deprecation which is only detectable at run-time.
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

impl ParseWarning {
    pub fn span(&self) -> Span {
        match self {
            ParseWarning::Deprecated { span, .. } => *span,
        }
    }
}

impl Reportable for ParseWarning {
    fn report_mode(&self) -> ReportMode {
        match self {
            ParseWarning::Deprecated { report_mode, .. } => *report_mode,
        }
    }
}

// To keep track of reported warnings
impl Hash for ParseWarning {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ParseWarning::Deprecated {
                dep_type, label, ..
            } => {
                dep_type.hash(state);
                label.hash(state);
            }
        }
    }
}
