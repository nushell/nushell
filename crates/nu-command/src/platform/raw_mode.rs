use nu_protocol::{ShellError, Span, engine::Stack, shell_error::io::IoError};

/// RAII guard for crossterm raw mode: [`acquire`](Self::acquire) checks
/// [`Stack::require_stdin`] and enables raw mode; [`Drop`] restores the previous
/// raw-mode state on any exit path.
///
/// Nested use (for example `input list` while reedline already has raw mode on)
/// must not `disable_raw_mode` on drop — that leaves the line editor cooked and
/// breaks backspace. If raw mode was already enabled, drop re-applies it.
#[must_use = "dropping the guard restores the previous raw-mode state"]
pub(crate) struct RawModeGuard {
    /// When true, the terminal was already in raw mode; leave it that way.
    leave_raw: bool,
}

impl RawModeGuard {
    /// Enter raw mode, or error per [`Stack::require_stdin`]. `span` points at the offending
    /// call.
    pub(crate) fn acquire(stack: &Stack, span: Span) -> Result<Self, ShellError> {
        stack.require_stdin(span)?;
        Self::enter(span)
    }

    /// Enable raw mode without the stdin check. Caller must have already called
    /// [`Stack::require_stdin`] if this context can be detached from the terminal.
    pub(crate) fn enter(span: Span) -> Result<Self, ShellError> {
        let was_raw = crossterm::terminal::is_raw_mode_enabled()
            .map_err(|err| IoError::new(err, span, None))?;
        if !was_raw {
            crossterm::terminal::enable_raw_mode().map_err(|err| IoError::new(err, span, None))?;
        }
        Ok(Self { leave_raw: was_raw })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.leave_raw {
            reapply_raw_mode();
        } else {
            let _ = crossterm::terminal::disable_raw_mode();
        }
    }
}

/// Crossterm's Unix `enable_raw_mode` is a no-op when it already recorded raw
/// mode, so it will not `tcsetattr` after fzf. Disable first to force a re-apply.
fn reapply_raw_mode() {
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::terminal::enable_raw_mode();
}
