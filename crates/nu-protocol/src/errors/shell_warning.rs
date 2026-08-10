#![allow(unused_assignments)]
use crate::Span;
use miette::Diagnostic;
use std::hash::Hash;
use thiserror::Error;

use crate::{ConfigWarning, ReportMode, Reportable};

#[derive(Clone, Debug, Error, Diagnostic)]
#[diagnostic(severity(Warning))]
pub enum ShellWarning {
    /// A parse-time deprecation. Indicates that something will be removed in a future release.
    ///
    /// Use [`ParseWarning::Deprecated`](crate::ParseWarning::Deprecated) if this is a deprecation
    /// which is detectable at parse-time.
    #[error("{dep_type} deprecated.")]
    #[diagnostic(code(nu::shell::deprecated))]
    Deprecated {
        dep_type: String,
        label: String,
        #[label("{label}")]
        span: Span,
        #[help]
        help: Option<String>,
        report_mode: ReportMode,
    },
    /// Warnings reported while updating the config
    #[error("Encountered {} warnings(s) when updating config", warnings.len())]
    #[diagnostic(code(nu::shell::invalid_config))]
    InvalidConfig {
        #[related]
        warnings: Vec<ConfigWarning>,
    },
    /// The interactive last-result value was truncated to fit `last_result_size`.
    ///
    /// Once-per-store is controlled by a stack flag; use [`ReportMode::EveryUse`] so the
    /// engine report log does not permanently suppress later truncations at the same limit.
    #[error(
        "Last result was truncated to fit $env.config.last_result_size ({limit_bytes} bytes by Value::memory_size)."
    )]
    #[diagnostic(code(nu::shell::last_result_truncated))]
    LastResultTruncated {
        /// Access site span (not used as a source label — truncation is a state warning).
        span: Span,
        limit_bytes: usize,
        #[help]
        help: Option<String>,
        report_mode: ReportMode,
    },
}

impl Reportable for ShellWarning {
    fn report_mode(&self) -> ReportMode {
        match self {
            ShellWarning::Deprecated { report_mode, .. }
            | ShellWarning::LastResultTruncated { report_mode, .. } => *report_mode,
            ShellWarning::InvalidConfig { .. } => ReportMode::FirstUse,
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
            // We always report config warnings, so no hash necessary
            ShellWarning::InvalidConfig { .. } => (),
            // EveryUse — hash unused for suppression; include fields for completeness
            ShellWarning::LastResultTruncated { limit_bytes, .. } => {
                limit_bytes.hash(state);
            }
        }
    }
}
