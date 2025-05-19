//! This module manages the step of turning error types into printed error messages
//!
//! Relies on the `miette` crate for pretty layout
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};

use crate::{
    engine::{EngineState, StateWorkingSet},
    CompileError, DeclId, ErrorStyle, ParseError, ParseWarning, ShellError,
};
use miette::{
    LabeledSpan, MietteHandlerOpts, NarratableReportHandler, ReportHandler, RgbColors, Severity,
    SourceCode,
};
use thiserror::Error;

/// The thread-global log of reported errors and warnings, so specific reports are only shown once
static REPORT_LOG: LazyLock<Mutex<ReportLog>> = LazyLock::new(Mutex::default);

/// This error exists so that we can defer SourceCode handling. It simply
/// forwards most methods, except for `.source_code()`, which we provide.
#[derive(Error)]
#[error("{0}")]
struct CliError<'src>(
    pub &'src dyn miette::Diagnostic,
    pub &'src StateWorkingSet<'src>,
);

#[derive(Default)]
struct ReportLog {
    deprecation_warnings: HashSet<DeclId>,
}

/// Returns true if this warning should be reported
fn should_show_warning(warning: &ParseWarning) -> bool {
    if let Some(decl_id) = match warning {
        ParseWarning::DeprecatedWarning { decl_id, .. } => Some(decl_id),
        ParseWarning::DeprecatedWarningWithMessage { decl_id, .. } => Some(decl_id),
        // this is unreachable right now, but future additions to ParseWarning shouldn't be matched here
        #[allow(unreachable_patterns)]
        _ => None,
    } {
        let is_first_report = REPORT_LOG
            .lock()
            .expect("report log is poisioned")
            .deprecation_warnings
            .insert(*decl_id);
        is_first_report
    } else {
        true
    }
}

pub fn format_shell_error(working_set: &StateWorkingSet, error: &ShellError) -> String {
    format!("Error: {:?}", CliError(error, working_set))
}

pub fn report_shell_error(engine_state: &EngineState, error: &ShellError) {
    if engine_state.config.display_errors.should_show(error) {
        report_error(&StateWorkingSet::new(engine_state), error)
    }
}

pub fn report_shell_warning(engine_state: &EngineState, warning: &ShellError) {
    if engine_state.config.display_errors.should_show(warning) {
        report_warning(&StateWorkingSet::new(engine_state), warning)
    }
}

pub fn report_parse_error(working_set: &StateWorkingSet, error: &ParseError) {
    report_error(working_set, error);
}

pub fn report_parse_warning(working_set: &StateWorkingSet, warning: &ParseWarning) {
    if should_show_warning(warning) {
        report_warning(working_set, warning);
    }
}

pub fn report_compile_error(working_set: &StateWorkingSet, error: &CompileError) {
    report_error(working_set, error);
}

fn report_error(working_set: &StateWorkingSet, error: &dyn miette::Diagnostic) {
    eprintln!("Error: {:?}", CliError(error, working_set));
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = nu_utils::enable_vt_processing();
    }
}

fn report_warning(working_set: &StateWorkingSet, warning: &dyn miette::Diagnostic) {
    eprintln!("Warning: {:?}", CliError(warning, working_set));
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = nu_utils::enable_vt_processing();
    }
}

impl std::fmt::Debug for CliError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config = self.1.get_config();

        let ansi_support = config.use_ansi_coloring.get(self.1.permanent());

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
        self.0.code()
    }

    fn severity(&self) -> Option<Severity> {
        self.0.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.0.url()
    }

    fn labels<'a>(&'a self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + 'a>> {
        self.0.labels()
    }

    // Finally, we redirect the source_code method to our own source.
    fn source_code(&self) -> Option<&dyn SourceCode> {
        if let Some(source_code) = self.0.source_code() {
            Some(source_code)
        } else {
            Some(&self.1)
        }
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        self.0.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
        self.0.diagnostic_source()
    }
}
