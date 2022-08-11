use crate::engine::StateWorkingSet;
use miette::{LabeledSpan, MietteHandlerOpts, ReportHandler, RgbColors, Severity, SourceCode};
use thiserror::Error;

/// This error exists so that we can defer SourceCode handling. It simply
/// forwards most methods, except for `.source_code()`, which we provide.
#[derive(Error)]
#[error("{0}")]
pub struct CliError<'src>(
    pub &'src (dyn miette::Diagnostic + Send + Sync + 'static),
    pub &'src StateWorkingSet<'src>,
);

pub fn format_error(
    working_set: &StateWorkingSet,
    error: &(dyn miette::Diagnostic + Send + Sync + 'static),
) -> String {
    format!("Error: {:?}", CliError(error, working_set))
}

impl std::fmt::Debug for CliError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ansi_support = self.1.get_config().use_ansi_coloring;

        let miette_handler = MietteHandlerOpts::new()
            // For better support of terminal themes use the ANSI coloring
            .rgb_colors(RgbColors::Never)
            // If ansi support is disabled in the config disable the eye-candy
            .color(ansi_support)
            .unicode(ansi_support)
            .terminal_links(ansi_support)
            .build();

        // Ignore error to prevent format! panics. This can happen if span points at some
        // inaccessible location, for example by calling `report_error()` with wrong working set.
        let _ = miette_handler.debug(self, f);

        Ok(())
    }
}

impl<'src> miette::Diagnostic for CliError<'src> {
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
}
