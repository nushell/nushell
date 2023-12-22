use std::{
    io,
    process::{Child, Command},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

/// A simple wrapper for `std::process::Command`
///
/// ## Spawn behavior
/// ### Unix
///
/// For interactive shells, the spawned child process will get its own process group id,
/// and it will be put in the foreground (by making stdin belong to the child's process group).
/// On drop, the calling process's group will become the foreground process group once again.
///
/// For non-interactive mode, processes are spawned normally without any foreground process handling.
///
/// ### Windows
///
/// It does nothing special on Windows systems, `spawn` is the same as [`std::process::Command::spawn`]
pub struct ForegroundProcess {
    inner: Command,
    _pipeline_state: Arc<(AtomicU32, AtomicU32)>,
}

/// A simple wrapper for `std::process::Child`
///
/// It can only be created by `ForegroundProcess::spawn`.
pub struct ForegroundChild {
    inner: Child,
    _pipeline_state: Option<Arc<(AtomicU32, AtomicU32)>>,
}

impl ForegroundProcess {
    pub fn new(cmd: Command, pipeline_state: Arc<(AtomicU32, AtomicU32)>) -> Self {
        Self {
            inner: cmd,
            _pipeline_state: pipeline_state,
        }
    }

    fn spawn_simple(&mut self) -> io::Result<ForegroundChild> {
        self.inner.spawn().map(|child| ForegroundChild {
            inner: child,
            _pipeline_state: None,
        })
    }

    #[cfg(not(unix))]
    pub fn spawn(&mut self, _interactive: bool) -> io::Result<ForegroundChild> {
        self.spawn_simple()
    }

    #[cfg(unix)]
    pub fn spawn(&mut self, interactive: bool) -> io::Result<ForegroundChild> {
        use std::io::IsTerminal;

        if interactive && io::stdin().is_terminal() {
            let (ref pgrp, ref pcnt) = *self._pipeline_state;
            let existing_pgrp = pgrp.load(Ordering::SeqCst);
            foreground_pgroup::prepare_command(&mut self.inner, existing_pgrp);
            self.inner
                .spawn()
                .map(|child| {
                    foreground_pgroup::set(&child, existing_pgrp);
                    let _ = pcnt.fetch_add(1, Ordering::SeqCst);
                    if existing_pgrp == 0 {
                        pgrp.store(child.id(), Ordering::SeqCst);
                    }
                    ForegroundChild {
                        inner: child,
                        _pipeline_state: Some(self._pipeline_state.clone()),
                    }
                })
                .map_err(|e| {
                    foreground_pgroup::reset();
                    e
                })
        } else {
            self.spawn_simple()
        }
    }
}

impl AsMut<Child> for ForegroundChild {
    fn as_mut(&mut self) -> &mut Child {
        &mut self.inner
    }
}

#[cfg(unix)]
impl Drop for ForegroundChild {
    fn drop(&mut self) {
        if let Some((pgrp, pcnt)) = self._pipeline_state.as_deref() {
            if pcnt.fetch_sub(1, Ordering::SeqCst) == 1 {
                pgrp.store(0, Ordering::SeqCst);
                foreground_pgroup::reset()
            }
        }
    }
}

// It's a simpler version of fish shell's external process handling.
#[cfg(unix)]
mod foreground_pgroup {
    use nix::{
        libc,
        sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
        unistd::{self, Pid},
    };
    use std::{
        os::unix::prelude::CommandExt,
        process::{Child, Command},
    };

    pub(super) fn prepare_command(external_command: &mut Command, existing_pgrp: u32) {
        unsafe {
            // Safety:
            // POSIX only allows async-signal-safe functions to be called.
            // `sigaction` and `getpid` are async-signal-safe according to:
            // https://manpages.ubuntu.com/manpages/bionic/man7/signal-safety.7.html
            // Also, `set_foreground_pid` is async-signal-safe.
            external_command.pre_exec(move || {
                // When this callback is run, std::process has already:
                // - reset SIGPIPE to SIG_DFL

                // According to glibc's job control manual:
                // https://www.gnu.org/software/libc/manual/html_node/Launching-Jobs.html
                // This has to be done *both* in the parent and here in the child due to race conditions.
                set_foreground_pid(unistd::getpid(), existing_pgrp);

                // Reset signal handlers for child, sync with `terminal.rs`
                let default = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
                // SIGINT has special handling
                sigaction(Signal::SIGQUIT, &default).expect("signal default");
                // We don't support background jobs, so keep SIGTSTP blocked?
                // sigaction(Signal::SIGTSTP, &default).expect("signal default");
                sigaction(Signal::SIGTTIN, &default).expect("signal default");
                sigaction(Signal::SIGTTOU, &default).expect("signal default");

                // TODO: determine if this is necessary or not, since this breaks `rm` on macOS
                // sigaction(Signal::SIGCHLD, &ignore).expect("signal default");

                sigaction(Signal::SIGTERM, &default).expect("signal default");

                Ok(())
            });
        }
    }

    pub(super) fn set(process: &Child, existing_pgrp: u32) {
        set_foreground_pid(Pid::from_raw(process.id() as i32), existing_pgrp);
    }

    fn set_foreground_pid(pid: Pid, existing_pgrp: u32) {
        // Safety: needs to be async-signal-safe.
        // `setpgid` and `tcsetpgrp` are async-signal-safe.

        // `existing_pgrp` is 0 when we don't have an existing foreground process in the pipeline.
        // A pgrp of 0 means the calling process's pid for `setpgid`. But not for `tcsetpgrp`.
        let pgrp = if existing_pgrp == 0 {
            pid
        } else {
            Pid::from_raw(existing_pgrp as i32)
        };
        let _ = unistd::setpgid(pid, pgrp);
        let _ = unistd::tcsetpgrp(libc::STDIN_FILENO, pgrp);
    }

    /// Reset the foreground process group to the shell
    pub(super) fn reset() {
        if let Err(e) = unistd::tcsetpgrp(libc::STDIN_FILENO, unistd::getpgrp()) {
            println!("ERROR: reset foreground id failed, tcsetpgrp result: {e:?}");
        }
    }
}
