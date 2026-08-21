use nu_protocol::{ShellError, Span, engine::Stack, shell_error::io::IoError};

/// RAII guard for crossterm raw mode: [`acquire`](Self::acquire) checks
/// [`Stack::require_stdin`] and enables raw mode; [`Drop`] disables it on any exit path.
#[must_use = "raw mode is disabled as soon as the guard is dropped"]
pub(crate) struct RawModeGuard;

impl RawModeGuard {
    /// Enter raw mode, or error per [`Stack::require_stdin`]. `span` points at the offending
    /// call.
    pub(crate) fn acquire(stack: &Stack, span: Span) -> Result<Self, ShellError> {
        stack.require_stdin(span)?;
        crossterm::terminal::enable_raw_mode().map_err(|err| IoError::new(err, span, None))?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}
