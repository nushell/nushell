#![allow(unused_assignments)]
use crate::Span;
use miette::Diagnostic;
use std::hash::Hash;
use thiserror::Error;

use crate::{ConfigWarning, ReportMode, Reportable};

#[derive(Clone, Debug, Error, Diagnostic)]
#[diagnostic(severity(Warning))]
pub enum ShellWarning {
    /// Running jobs (without auto-kill) prevented interactive shell from exiting.
    #[error("Running background jobs that won't be killed automatically: {job_count}.")]
    #[diagnostic(
        code(nu::shell::background_jobs_running),
        help(
            "Run `exit` again to kill all jobs.\nUse `job spawn --auto-kill` when spawning thread jobs to exclude them from this warning."
        )
    )]
    BackgroundJobsRunning { job_count: usize },
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
    /// The interactive last-result value was truncated to fit `max_last_result_size`.
    ///
    /// Once-per-store is controlled by a stack flag; use [`ReportMode::EveryUse`] so the
    /// engine report log does not permanently suppress later truncations at the same limit.
    #[error(
        "Last result was truncated to fit $env.config.max_last_result_size ({limit_bytes} bytes by Value::memory_size)."
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
    /// The interactive last-result value stopped short because its stream wasn't read to the end,
    /// which raising `max_last_result_size` wouldn't change.
    ///
    /// "May be" because a reader that stops right after the last item can't tell it was the last.
    /// Reported like [`Self::LastResultTruncated`].
    #[error("Last result may be incomplete because its stream was not read to the end.")]
    #[diagnostic(
        code(nu::shell::last_result_incomplete),
        help(
            "`${}.last` holds what was read before the stream stopped. Ctrl-C, an error, or a `display_output` hook that stops reading can end it early.",
            crate::LAST_RESULT_VAR_NAME
        )
    )]
    LastResultIncomplete,
}

impl Reportable for ShellWarning {
    fn report_mode(&self) -> ReportMode {
        match self {
            ShellWarning::Deprecated { report_mode, .. }
            | ShellWarning::LastResultTruncated { report_mode, .. } => *report_mode,
            ShellWarning::InvalidConfig { .. } => ReportMode::FirstUse,
            ShellWarning::BackgroundJobsRunning { .. } => ReportMode::EveryUse,
            // Once-per-access is handled by a stack flag.
            ShellWarning::LastResultIncomplete => ReportMode::EveryUse,
        }
    }
}

// To keep track of reported warnings
impl Hash for ShellWarning {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ShellWarning::BackgroundJobsRunning { job_count } => job_count.hash(state),
            ShellWarning::Deprecated {
                dep_type, label, ..
            } => {
                dep_type.hash(state);
                label.hash(state);
            }
            // Hash the contents so FirstUse dedups per warning batch, not
            // once for all config warnings in the session.
            ShellWarning::InvalidConfig { warnings } => warnings.hash(state),
            // EveryUse — hash unused for suppression; include fields for completeness
            ShellWarning::LastResultTruncated { limit_bytes, .. } => {
                limit_bytes.hash(state);
            }
            ShellWarning::LastResultIncomplete => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConfigWarning;

    fn hash_of(warning: &ShellWarning) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        warning.hash(&mut hasher);
        std::hash::Hasher::finish(&hasher)
    }

    fn incompatible_batch(label: &'static str) -> ShellWarning {
        ShellWarning::InvalidConfig {
            warnings: vec![ConfigWarning::IncompatibleOptions {
                label,
                span: Span::test_data(),
                help: "",
            }],
        }
    }

    /// `report_mode` is `FirstUse`, which dedups by this hash; a constant hash
    /// would suppress every config warning after the first batch of a session.
    #[test]
    fn different_config_warning_batches_hash_differently() {
        assert_ne!(
            hash_of(&incompatible_batch("history")),
            hash_of(&incompatible_batch("other")),
        );
    }

    /// The same batch re-reported (e.g. a re-sourced config) stays suppressed.
    #[test]
    fn an_identical_config_warning_batch_hashes_the_same() {
        assert_eq!(
            hash_of(&incompatible_batch("history")),
            hash_of(&incompatible_batch("history")),
        );
    }
}
