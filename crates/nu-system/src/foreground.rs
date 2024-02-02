use std::{
    io,
    process::{Child, Command},
};

#[cfg(unix)]
use std::{
    io::IsTerminal,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

/// A simple wrapper for [`std::process::Child`]
///
/// It can only be created by [`ForegroundChild::spawn`].
///
/// # Spawn behavior
/// ## Unix
///
/// For interactive shells, the spawned child process will get its own process group id,
/// and it will be put in the foreground (by making stdin belong to the child's process group).
/// On drop, the calling process's group will become the foreground process group once again.
///
/// For non-interactive mode, processes are spawned normally without any foreground process handling.
///
/// ## Other systems
///
/// It does nothing special on non-unix systems, so `spawn` is the same as [`std::process::Command::spawn`].
pub struct ForegroundChild {
    inner: Child,
    #[cfg(unix)]
    pipeline_state: Option<Arc<(AtomicU32, AtomicU32)>>,
}

impl ForegroundChild {
    #[cfg(not(unix))]
    pub fn spawn(mut command: Command) -> io::Result<Self> {
        command.spawn().map(|child| Self { inner: child })
    }

    #[cfg(unix)]
    pub fn spawn(
        mut command: Command,
        interactive: bool,
        pipeline_state: &Arc<(AtomicU32, AtomicU32)>,
    ) -> io::Result<Self> {
        if interactive && io::stdin().is_terminal() {
            let (pgrp, pcnt) = pipeline_state.as_ref();
            let existing_pgrp = pgrp.load(Ordering::SeqCst);
            foreground_pgroup::prepare_command(&mut command, existing_pgrp);
            command
                .spawn()
                .map(|child| {
                    foreground_pgroup::set(&child, existing_pgrp);
                    let _ = pcnt.fetch_add(1, Ordering::SeqCst);
                    if existing_pgrp == 0 {
                        pgrp.store(child.id(), Ordering::SeqCst);
                    }
                    Self {
                        inner: child,
                        pipeline_state: Some(pipeline_state.clone()),
                    }
                })
                .map_err(|e| {
                    foreground_pgroup::reset();
                    e
                })
        } else {
            command.spawn().map(|child| Self {
                inner: child,
                pipeline_state: None,
            })
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
        if let Some((pgrp, pcnt)) = self.pipeline_state.as_deref() {
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

    pub fn prepare_command(external_command: &mut Command, existing_pgrp: u32) {
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
                set_foreground_pid(Pid::this(), existing_pgrp);

                // Reset signal handlers for child, sync with `terminal.rs`
                let default = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
                // SIGINT has special handling
                let _ = sigaction(Signal::SIGQUIT, &default);
                // We don't support background jobs, so keep some signals blocked for now
                // let _ = sigaction(Signal::SIGTSTP, &default);
                // let _ = sigaction(Signal::SIGTTIN, &default);
                // let _ = sigaction(Signal::SIGTTOU, &default);
                let _ = sigaction(Signal::SIGTERM, &default);

                Ok(())
            });
        }
    }

    pub fn set(process: &Child, existing_pgrp: u32) {
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
    pub fn reset() {
        if let Err(e) = unistd::tcsetpgrp(libc::STDIN_FILENO, unistd::getpgrp()) {
            eprintln!("ERROR: reset foreground id failed, tcsetpgrp result: {e:?}");
        }
    }
}
