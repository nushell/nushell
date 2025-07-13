//! This module manages the step of turning error types into printed error messages
//!
//! Relies on the `miette` crate for pretty layout
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{
    CompileError, ErrorStyle, ParseError, ParseWarning, ShellError,
    engine::{EngineState, StateWorkingSet},
};
use miette::{
    LabeledSpan, MietteHandlerOpts, NarratableReportHandler, ReportHandler, RgbColors, Severity,
    SourceCode,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// This error exists so that we can defer SourceCode handling. It simply
/// forwards most methods, except for `.source_code()`, which we provide.
#[derive(Error)]
#[error("{diagnostic}")]
struct CliError<'src> {
    diagnostic: &'src dyn miette::Diagnostic,
    working_set: &'src StateWorkingSet<'src>,
    // error code to use if `diagnostic` doesn't provide one
    default_code: Option<&'static str>,
}

impl<'src> CliError<'src> {
    pub fn new(
        diagnostic: &'src dyn miette::Diagnostic,
        working_set: &'src StateWorkingSet<'src>,
        default_code: Option<&'static str>,
    ) -> Self {
        CliError {
            diagnostic,
            working_set,
            default_code,
        }
    }
}

#[derive(Default)]
pub struct ReportLog {
    // A bloom-filter like structure to store the hashes of `ParseWarning`s,
    // without actually permanently storing the entire warning in memory.
    // May rarely result in warnings incorrectly being unreported upon hash collision.
    parse_warnings: Vec<u64>,
}

/// How a warning/error should be reported
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReportMode {
    FirstUse,
    EveryUse,
}

/// Returns true if this warning should be reported
fn should_show_warning(engine_state: &EngineState, warning: &ParseWarning) -> bool {
    match warning.report_mode() {
        ReportMode::EveryUse => true,
        ReportMode::FirstUse => {
            let mut hasher = DefaultHasher::new();
            warning.hash(&mut hasher);
            let hash = hasher.finish();

            let mut report_log = engine_state
                .report_log
                .lock()
                .expect("report log lock is poisioned");

            match report_log.parse_warnings.contains(&hash) {
                true => false,
                false => {
                    report_log.parse_warnings.push(hash);
                    true
                }
            }
        }
    }
}

pub fn format_cli_error(working_set: &StateWorkingSet, error: &dyn miette::Diagnostic) -> String {
    format!("Error: {:?}", CliError::new(error, working_set, None))
}

pub fn report_shell_error(engine_state: &EngineState, error: &ShellError) {
    if engine_state.config.display_errors.should_show(error) {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(&working_set, error, "nu::shell::error")
    }
}

pub fn report_shell_warning(engine_state: &EngineState, warning: &ShellError) {
    if engine_state.config.display_errors.should_show(warning) {
        report_warning(
            &StateWorkingSet::new(engine_state),
            warning,
            "nu::shell::warning",
        )
    }
}

pub fn report_parse_error(working_set: &StateWorkingSet, error: &ParseError) {
    report_error(working_set, error, "nu::parser::error");
}

pub fn report_parse_warning(working_set: &StateWorkingSet, warning: &ParseWarning) {
    if should_show_warning(working_set.permanent(), warning) {
        report_warning(working_set, warning, "nu::parser::warning");
    }
}

pub fn report_compile_error(working_set: &StateWorkingSet, error: &CompileError) {
    report_error(working_set, error, "nu::compile::error");
}

pub fn report_experimental_option_warning(
    working_set: &StateWorkingSet,
    warning: &dyn miette::Diagnostic,
) {
    report_warning(working_set, warning, "nu::experimental_option::warning");
}

fn report_error(
    working_set: &StateWorkingSet,
    error: &dyn miette::Diagnostic,
    default_code: &'static str,
) {
    eprintln!(
        "Error: {:?}",
        CliError::new(error, working_set, Some(default_code))
    );
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = nu_utils::enable_vt_processing();
    }
}

fn report_warning(
    working_set: &StateWorkingSet,
    warning: &dyn miette::Diagnostic,
    default_code: &'static str,
) {
    eprintln!(
        "Warning: {:?}",
        CliError::new(warning, working_set, Some(default_code))
    );
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = nu_utils::enable_vt_processing();
    }
}

impl std::fmt::Debug for CliError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config = self.working_set.get_config();

        let ansi_support = config.use_ansi_coloring.get(self.working_set.permanent());

        let error_style = &config.error_style;

        let miette_handler: Box<dyn ReportHandler> = match error_style {
            ErrorStyle::Plain => Box::new(NarratableReportHandler::new()),
            ErrorStyle::Fancy => Box::new(
                MietteHandlerOpts::new()
                    // For better support of terminal themes use the ANSI coloring
                    .rgb_colors(RgbColors::Never)
                    // If ansi support is disabled in the config disable the eye-candy
                    .color(ansi_support)
                    .unicode(ansi_support)
                    .terminal_links(ansi_support)
                    .build(),
            ),
        };

        // Ignore error to prevent format! panics. This can happen if span points at some
        // inaccessible location, for example by calling `report_error()` with wrong working set.
        let _ = miette_handler.debug(self, f);

        Ok(())
    }
}

impl miette::Diagnostic for CliError<'_> {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.diagnostic.code().or_else(|| {
            self.default_code
                .map(|code| Box::new(code) as Box<dyn std::fmt::Display>)
        })
    }

    fn severity(&self) -> Option<Severity> {
        self.diagnostic.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.diagnostic.help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.diagnostic.url()
    }

    fn labels<'a>(&'a self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + 'a>> {
        self.diagnostic.labels()
    }

    // Finally, we redirect the source_code method to our own source.
    fn source_code(&self) -> Option<&dyn SourceCode> {
        if let Some(source_code) = self.diagnostic.source_code() {
            Some(source_code)
        } else {
            Some(&self.working_set)
        }
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        self.diagnostic.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
        self.diagnostic.diagnostic_source()
    }
}
