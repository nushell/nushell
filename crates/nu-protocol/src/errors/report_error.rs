//! This module manages the step of turning error types into printed error messages
//!
//! Relies on the `miette` crate for pretty layout
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Write;

use crate::{
    CompileError, Config, ErrorStyle, ParseError, ParseWarning, ShellError, ShellWarning,
    ShortReportHandler,
    engine::{EngineState, Stack, StateWorkingSet},
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
    stack: Option<&'src Stack>,
    diagnostic: &'src dyn miette::Diagnostic,
    working_set: &'src StateWorkingSet<'src>,
    // error code to use if `diagnostic` doesn't provide one
    default_code: Option<&'static str>,
}

impl<'src> CliError<'src> {
    pub fn new(
        stack: Option<&'src Stack>,
        diagnostic: &'src dyn miette::Diagnostic,
        working_set: &'src StateWorkingSet<'src>,
        default_code: Option<&'static str>,
    ) -> Self {
        CliError {
            stack,
            diagnostic,
            working_set,
            default_code,
        }
    }
}

/// A bloom-filter like structure to store the hashes of warnings,
/// without actually permanently storing the entire warning in memory.
/// May rarely result in warnings incorrectly being unreported upon hash collision.
#[derive(Default)]
pub struct ReportLog(Vec<u64>);

/// How a warning/error should be reported
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReportMode {
    FirstUse,
    EveryUse,
}

/// For warnings/errors which have a ReportMode that dictates when they are reported
pub trait Reportable {
    fn report_mode(&self) -> ReportMode;
}

/// Returns true if this warning should be reported
fn should_show_reportable<R>(engine_state: &EngineState, reportable: &R) -> bool
where
    R: Reportable + Hash,
{
    match reportable.report_mode() {
        ReportMode::EveryUse => true,
        ReportMode::FirstUse => {
            let mut hasher = DefaultHasher::new();
            reportable.hash(&mut hasher);
            let hash = hasher.finish();

            let mut report_log = engine_state
                .report_log
                .lock()
                .expect("report log lock is poisoned");

            match report_log.0.contains(&hash) {
                true => false,
                false => {
                    report_log.0.push(hash);
                    true
                }
            }
        }
    }
}

pub fn format_cli_error(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    error: &dyn miette::Diagnostic,
    default_code: Option<&'static str>,
) -> String {
    format!(
        "Error: {:?}",
        CliError::new(stack, error, working_set, default_code)
    )
}

pub fn report_shell_error(stack: Option<&Stack>, engine_state: &EngineState, error: &ShellError) {
    if get_config(stack, engine_state)
        .display_errors
        .should_show(error)
    {
        let working_set = StateWorkingSet::new(engine_state);
        report_error(stack, &working_set, error, "nu::shell::error")
    }
}

pub fn report_shell_warning(
    stack: Option<&Stack>,
    engine_state: &EngineState,
    warning: &ShellWarning,
) {
    if should_show_reportable(engine_state, warning) {
        report_warning(
            stack,
            &StateWorkingSet::new(engine_state),
            warning,
            "nu::shell::warning",
        );
    }
}

pub fn report_parse_error(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    error: &ParseError,
) {
    report_error(stack, working_set, error, "nu::parser::error");
}

pub fn report_parse_warning(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    warning: &ParseWarning,
) {
    if should_show_reportable(working_set.permanent(), warning) {
        report_warning(stack, working_set, warning, "nu::parser::warning");
    }
}

pub fn report_compile_error(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    error: &CompileError,
) {
    report_error(stack, working_set, error, "nu::compile::error");
}

pub fn report_experimental_option_warning(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    warning: &dyn miette::Diagnostic,
) {
    report_warning(
        stack,
        working_set,
        warning,
        "nu::experimental_option::warning",
    );
}

fn report_error(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    error: &dyn miette::Diagnostic,
    default_code: &'static str,
) {
    // Avoid eprintln! since it panics on broken stderr, which double-panics
    // through miette's panic hook and aborts.
    let _ = writeln!(
        std::io::stderr(),
        "Error: {:?}",
        CliError::new(stack, error, working_set, Some(default_code))
    );
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = nu_utils::enable_vt_processing();
    }
}

fn report_warning(
    stack: Option<&Stack>,
    working_set: &StateWorkingSet,
    warning: &dyn miette::Diagnostic,
    default_code: &'static str,
) {
    let _ = writeln!(
        std::io::stderr(),
        "Warning: {:?}",
        CliError::new(stack, warning, working_set, Some(default_code))
    );
    // reset vt processing, aka ansi because illbehaved externals can break it
    #[cfg(windows)]
    {
        let _ = nu_utils::enable_vt_processing();
    }
}

fn get_config<'a>(stack: Option<&'a Stack>, engine_state: &'a EngineState) -> &'a Config {
    stack
        .and_then(|s| s.config.as_deref())
        .unwrap_or(engine_state.get_config())
}

impl std::fmt::Debug for CliError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let engine_state = self.working_set.permanent();
        let config = get_config(self.stack, engine_state);

        let ansi_support = config.use_ansi_coloring.get(engine_state);

        let error_style = config.error_style;

        let error_lines = config.error_lines;

        let miette_handler: Box<dyn ReportHandler> = match error_style {
            ErrorStyle::Short => Box::new(ShortReportHandler::new()),
            ErrorStyle::Plain => Box::new(NarratableReportHandler::new()),
            style => {
                let handler = MietteHandlerOpts::new()
                    // For better support of terminal themes use the ANSI coloring
                    .rgb_colors(RgbColors::Never)
                    // If ansi support is disabled in the config disable the eye-candy
                    .color(ansi_support)
                    .unicode(ansi_support)
                    .terminal_links(ansi_support)
                    .context_lines(error_lines as usize);
                match style {
                    ErrorStyle::Nested => Box::new(
                        handler
                            .show_related_errors_as_nested()
                            .with_cause_chain()
                            .build(),
                    ),
                    _ => Box::new(handler.build()),
                }
            }
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
