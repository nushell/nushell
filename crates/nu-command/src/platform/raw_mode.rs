use nu_protocol::{ShellError, Span, engine::Stack, shell_error::io::IoError};

/// RAII guard for crossterm raw mode: [`acquire`](Self::acquire) checks
/// [`Stack::require_stdin`] and enables raw mode; [`Drop`] restores the previous
/// raw-mode state on any exit path.
///
/// Output processing stays on in raw mode (see [`enable_raw_mode`]), so a background job that
/// prints while the guard is held doesn't stairstep.
///
/// Nested use (for example `input list` while reedline already has raw mode on)
/// must not `disable_raw_mode` on drop — that leaves the line editor cooked and
/// breaks backspace. If raw mode was already enabled, drop re-applies it.
#[must_use = "dropping the guard restores the previous raw-mode state"]
pub struct RawModeGuard {
    /// When true, the terminal was already in raw mode; leave it that way.
    leave_raw: bool,
}

impl RawModeGuard {
    /// Enter raw mode, or error per [`Stack::require_stdin`]. `span` points at the offending
    /// call.
    pub fn acquire(stack: &Stack, span: Span) -> Result<Self, ShellError> {
        stack.require_stdin(span)?;
        Self::enter(span)
    }

    /// Enable raw mode without the stdin check. Caller must have already called
    /// [`Stack::require_stdin`] if this context can be detached from the terminal.
    pub fn enter(span: Span) -> Result<Self, ShellError> {
        let was_raw = crossterm::terminal::is_raw_mode_enabled()
            .map_err(|err| IoError::new(err, span, None))?;
        if !was_raw {
            enable_raw_mode().map_err(|err| IoError::new(err, span, None))?;
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
    let _ = enable_raw_mode();
}

/// Enable crossterm's raw mode with output processing left on.
///
/// On Unix, crossterm's raw mode also clears `OPOST`, the terminal setting that writes `\n` as
/// `\r\n`. Text a background job prints while raw mode is on would then start each line where the
/// last one ended. readline, zsh and reedline keep output processing on in raw mode, and so does
/// this. Leaving raw mode needs nothing extra: crossterm writes back the whole mode it saved.
fn enable_raw_mode() -> std::io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    // Raw mode works without it; only output from other threads stairsteps.
    #[cfg(unix)]
    if let Some(tty) = tty() {
        let _ = keep_output_processing(tty);
    }
    Ok(())
}

/// The terminal crossterm switches into raw mode: stdin if it is a terminal, else `/dev/tty`.
#[cfg(unix)]
fn tty() -> Option<std::os::fd::OwnedFd> {
    use std::{io::IsTerminal, os::fd::AsFd};

    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        stdin.as_fd().try_clone_to_owned().ok()
    } else {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .ok()
            .map(Into::into)
    }
}

/// Turn output processing back on for `tty`, with `\n` written as `\r\n`.
#[cfg(unix)]
fn keep_output_processing(tty: impl std::os::fd::AsFd) -> nix::Result<()> {
    use nix::sys::termios::{OutputFlags, SetArg, tcgetattr, tcsetattr};

    let mut termios = tcgetattr(&tty)?;
    termios.output_flags |= OutputFlags::OPOST | OutputFlags::ONLCR;
    // A `\r` written to move to the first column must stay a carriage return.
    termios.output_flags.remove(OutputFlags::OCRNL);
    tcsetattr(&tty, SetArg::TCSANOW, &termios)
}

#[cfg(all(test, unix))]
mod tests {
    use super::keep_output_processing;
    use nix::{
        pty::openpty,
        sys::termios::{SetArg, cfmakeraw, tcgetattr, tcsetattr},
    };
    use std::{
        fs::File,
        io::{Read, Write},
    };

    /// What `a\nb` written to a pseudo-terminal in crossterm's raw mode comes out as, with or
    /// without output processing turned back on.
    fn newline_through_raw_pty(keep_processing: bool) -> Vec<u8> {
        let pty = openpty(None, None).expect("opening a pseudo-terminal");
        // What crossterm's `enable_raw_mode` does on Unix.
        let mut termios = tcgetattr(&pty.slave).expect("reading the terminal mode");
        cfmakeraw(&mut termios);
        tcsetattr(&pty.slave, SetArg::TCSANOW, &termios).expect("entering raw mode");

        if keep_processing {
            keep_output_processing(&pty.slave).expect("turning output processing on");
        }

        // Keep the slave open until the master is read: once its last descriptor closes, Linux
        // may answer the read with EIO.
        let mut slave = File::from(pty.slave);
        slave
            .write_all(b"a\nb")
            .expect("writing to the pseudo-terminal");
        let mut out = [0; 16];
        let read = File::from(pty.master)
            .read(&mut out)
            .expect("reading the pseudo-terminal");
        out[..read].to_vec()
    }

    #[test]
    fn raw_mode_alone_writes_a_bare_line_feed() {
        assert_eq!(newline_through_raw_pty(false), b"a\nb");
    }

    #[test]
    fn kept_output_processing_writes_a_newline_as_crlf() {
        assert_eq!(newline_through_raw_pty(true), b"a\r\nb");
    }
}
