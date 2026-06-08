#![allow(unused_assignments)]
use crate::{ShellError, Span, Type};
use miette::Diagnostic;
use std::hash::Hash;
use thiserror::Error;

/// The errors that may occur when updating the config
#[derive(Clone, Debug, PartialEq, Error, Diagnostic)]
pub enum ConfigError {
    #[error("Type mismatch at {path}")]
    #[diagnostic(code(nu::shell::type_mismatch))]
    TypeMismatch {
        path: String,
        expected: Type,
        actual: Type,
        #[label = "expected {expected}, but got {actual}"]
        span: Span,
    },
    #[error("Invalid value for {path}")]
    #[diagnostic(code(nu::shell::invalid_value))]
    InvalidValue {
        path: String,
        valid: String,
        actual: String,
        #[label = "expected {valid}, but got {actual}"]
        span: Span,
    },
    #[error("Unknown config option: {path}")]
    #[diagnostic(code(nu::shell::unknown_config_option))]
    UnknownOption {
        path: String,
        #[label("remove this")]
        span: Span,
    },
    #[error("{path} requires a '{column}' column")]
    #[diagnostic(code(nu::shell::missing_required_column))]
    MissingRequiredColumn {
        path: String,
        column: &'static str,
        #[label("has no '{column}' column")]
        span: Span,
    },
    #[error("{path} is deprecated")]
    #[diagnostic(
        code(nu::shell::deprecated_config_option),
        help("please {suggestion} instead")
    )]
    Deprecated {
        path: String,
        suggestion: &'static str,
        #[label("deprecated")]
        span: Span,
    },
    #[error("{path} cannot be changed after Nushell has started")]
    #[diagnostic(
        code(nu::shell::config_option_locked_after_startup),
        help(
            "set {path} in your config.nu (or via --config / the relevant env var) and restart Nushell"
        )
    )]
    LockedAfterStartup {
        path: String,
        #[label("cannot be changed at runtime")]
        span: Span,
    },
    // TODO: remove this
    #[error(transparent)]
    #[diagnostic(transparent)]
    ShellError(#[from] ShellError),
}

/// Warnings which don't prevent config from being loaded, but we should inform the user about
#[derive(Clone, Debug, PartialEq, Error, Diagnostic)]
#[diagnostic(severity(Warning))]
pub enum ConfigWarning {
    #[error("Incompatible options")]
    #[diagnostic(code(nu::shell::incompatible_options), help("{help}"))]
    IncompatibleOptions {
        label: &'static str,
        #[label = "{label}"]
        span: Span,
        help: &'static str,
    },
}

// To keep track of reported warnings
impl Hash for ConfigWarning {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ConfigWarning::IncompatibleOptions { label, help, .. } => {
                label.hash(state);
                help.hash(state);
            }
        }
    }
}
