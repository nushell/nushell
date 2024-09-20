use crate::{ShellError, Span};
use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Exited(i32),
    #[cfg(unix)]
    Signaled {
        signal: i32,
        core_dumped: bool,
    },
}

impl ExitStatus {
    pub fn code(self) -> i32 {
        match self {
            ExitStatus::Exited(code) => code,
            #[cfg(unix)]
            ExitStatus::Signaled { signal, .. } => -signal,
        }
    }

    pub fn check_ok(self, ignore_error: bool, span: Span) -> Result<(), ShellError> {
        match self {
            ExitStatus::Exited(exit_code) => {
                if ignore_error {
                    Ok(())
                } else if let Ok(exit_code) = exit_code.try_into() {
                    Err(ShellError::NonZeroExitCode { exit_code, span })
                } else {
                    Ok(())
                }
            }
            #[cfg(unix)]
            ExitStatus::Signaled {
                signal,
                core_dumped,
            } => {
                use nix::sys::signal::Signal;

                let sig = Signal::try_from(signal);

                if sig == Ok(Signal::SIGPIPE) || (ignore_error && !core_dumped) {
                    // Processes often exit with SIGPIPE, but this is not an error condition.
                    Ok(())
                } else {
                    let signal_name = sig.map(Signal::as_str).unwrap_or("unknown signal").into();
                    Err(if core_dumped {
                        ShellError::CoreDumped {
                            signal_name,
                            signal,
                            span,
                        }
                    } else {
                        ShellError::TerminatedBySignal {
                            signal_name,
                            signal,
                            span,
                        }
                    })
                }
            }
        }
    }
}

#[cfg(unix)]
impl From<process::ExitStatus> for ExitStatus {
    fn from(status: process::ExitStatus) -> Self {
        use std::os::unix::process::ExitStatusExt;

        match (status.code(), status.signal()) {
            (Some(code), None) => Self::Exited(code),
            (None, Some(signal)) => Self::Signaled {
                signal,
                core_dumped: status.core_dumped(),
            },
            (None, None) => {
                debug_assert!(false, "ExitStatus should have either a code or a signal");
                Self::Exited(-1)
            }
            (Some(code), Some(signal)) => {
                // Should be unreachable, as `code()` will be `None` if `signal()` is `Some`
                // according to the docs for `ExitStatus::code`.
                debug_assert!(
                    false,
                    "ExitStatus cannot have both a code ({code}) and a signal ({signal})"
                );
                Self::Signaled {
                    signal,
                    core_dumped: status.core_dumped(),
                }
            }
        }
    }
}

#[cfg(not(unix))]
impl From<process::ExitStatus> for ExitStatus {
    fn from(status: process::ExitStatus) -> Self {
        let code = status.code();
        debug_assert!(
            code.is_some(),
            "`ExitStatus::code` cannot return `None` on windows"
        );
        Self::Exited(code.unwrap_or(-1))
    }
}
