use std::fmt;

use miette::{Diagnostic, ReportHandler};

/// A [`ReportHandler`] that renders errors as plain text without graphics.
/// Designed for concise output that typically fits on a single line.
#[derive(Debug, Clone)]
pub struct ShortReportHandler {}

impl ShortReportHandler {
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for ShortReportHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ShortReportHandler {
    /// Render a [`Diagnostic`]. This function is meant to be called
    /// by the toplevel [`ReportHandler`].
    fn render_report(
        &self,
        f: &mut fmt::Formatter<'_>,
        diagnostic: &dyn Diagnostic,
    ) -> fmt::Result {
        write!(f, "{}: ", diagnostic)?;

        if let Some(labels) = diagnostic.labels() {
            let mut labels = labels
                .into_iter()
                .filter_map(|span| span.label().map(String::from))
                .peekable();

            while let Some(label) = labels.next() {
                let end_char = if labels.peek().is_some() { ", " } else { " " };
                write!(f, "{}{}", label, end_char)?;
            }
        }

        if let Some(help) = diagnostic.help() {
            write!(f, "({})", help)?;
        }

        Ok(())
    }
}

impl ReportHandler for ShortReportHandler {
    fn debug(&self, diagnostic: &dyn Diagnostic, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.render_report(f, diagnostic)
    }
}
