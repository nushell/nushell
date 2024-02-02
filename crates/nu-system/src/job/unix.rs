use std::{
    io::{self, IsTerminal},
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Command, ExitStatus},
};

use nix::{
    sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
    unistd::{self, Pid},
};

use crate::{JobExitStatus, Jobs};

impl From<ExitStatus> for JobExitStatus {
    fn from(status: ExitStatus) -> Self {
        match (status.code(), status.signal()) {
            (Some(code), None) => Self::Exited(code),
            (None, Some(signal)) => Self::Signaled {
                signal,
                core_dumped: status.core_dumped(),
            },
            (None, None) => Self::Unknown,
            (Some(code), Some(signal)) => {
                // Should be unreachable, as `code()` will be `None` if `signal()` is `Some`
                // according to the docs for `ExitStatus::code`.
                debug_assert!(
                    false,
                    "ExitStatus cannot have both a code ({code}) and a signal ({signal})"
                );
                Self::Unknown
            }
        }
    }
}

impl Jobs {
    pub(crate) fn platform_pre_spawn(command: &mut Command, interactive: bool) {
        if interactive && io::stdin().is_terminal() {
            prepare_interactive(command);
        }
    }
}

fn prepare_interactive(command: &mut Command) {
    unsafe {
        // Safety:
        // POSIX only allows async-signal-safe functions to be called.
        // `sigaction`, `getpid`, `setpgid`, and `tcsetpgrp` are async-signal-safe according to:
        // https://manpages.ubuntu.com/manpages/bionic/man7/signal-safety.7.html
        command.pre_exec(move || {
            // When this callback is run, std::process has already:
            // - reset SIGPIPE to SIG_DFL

            let pid = Pid::this();

            // According to glibc's job control manual:
            // https://www.gnu.org/software/libc/manual/html_node/Launching-Jobs.html
            // This has to be done *both* in the parent and here in the child due to race conditions.
            let _ = unistd::setpgid(pid, pid);

            // Reset signal handlers for child, sync with `terminal.rs`
            let default = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
            // SIGINT has special handling
            let _ = sigaction(Signal::SIGQUIT, &default);
            let _ = sigaction(Signal::SIGTSTP, &default);
            let _ = sigaction(Signal::SIGTTIN, &default);
            let _ = sigaction(Signal::SIGTTOU, &default);
            let _ = sigaction(Signal::SIGTERM, &default);

            Ok(())
        });
    }
}
