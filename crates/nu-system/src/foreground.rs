use std::{
    io,
    process::{Child, Command},
    sync::{atomic::AtomicU32, Arc},
};

#[cfg(unix)]
use std::{io::IsTerminal, sync::atomic::Ordering};

#[cfg(unix)]
pub use foreground_pgroup::stdin_fd;

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

/// Keeps a specific already existing process in the foreground as long as the [`ForegroundGuard`].
/// If the process needs to be spawned in the foreground, use [`ForegroundChild`] instead. This is
/// used to temporarily bring plugin processes into the foreground.
///
/// # OS-specific behavior
/// ## Unix
///
/// If there is already a foreground external process running, spawned with [`ForegroundChild`],
/// this expects the process ID to remain in the process group created by the [`ForegroundChild`]
/// for the lifetime of the guard, and keeps the terminal controlling process group set to that. If
/// there is no foreground external process running, this sets the foreground process group to the
/// plugin's process ID. The process group that is expected can be retrieved with [`.pgrp()`] if
/// different from the plugin process ID.
///
/// ## Other systems
///
/// It does nothing special on non-unix systems.
#[derive(Debug)]
pub struct ForegroundGuard {
    #[cfg(unix)]
    pgrp: Option<u32>,
    #[cfg(unix)]
    pipeline_state: Arc<(AtomicU32, AtomicU32)>,
}

impl ForegroundGuard {
    /// Move the given process to the foreground.
    #[cfg(unix)]
    pub fn new(
        pid: u32,
        pipeline_state: &Arc<(AtomicU32, AtomicU32)>,
    ) -> std::io::Result<ForegroundGuard> {
        use nix::unistd::{self, Pid};

        let pid_nix = Pid::from_raw(pid as i32);
        let (pgrp, pcnt) = pipeline_state.as_ref();

        // Might have to retry due to race conditions on the atomics
        loop {
            // Try to give control to the child, if there isn't currently a foreground group
            if pgrp
                .compare_exchange(0, pid, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let _ = pcnt.fetch_add(1, Ordering::SeqCst);

                // We don't need the child to change process group. Make the guard now so that if there
                // is an error, it will be cleaned up
                let guard = ForegroundGuard {
                    pgrp: None,
                    pipeline_state: pipeline_state.clone(),
                };

                log::trace!("Giving control of the terminal to the plugin group, pid={pid}");

                // Set the terminal controlling process group to the child process
                unistd::tcsetpgrp(unsafe { stdin_fd() }, pid_nix)?;

                return Ok(guard);
            } else if pcnt
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                    // Avoid a race condition: only increment if count is > 0
                    if count > 0 {
                        Some(count + 1)
                    } else {
                        None
                    }
                })
                .is_ok()
            {
                // We successfully added another count to the foreground process group, which means
                // we only need to tell the child process to join this one
                let pgrp = pgrp.load(Ordering::SeqCst);
                log::trace!(
                    "Will ask the plugin pid={pid} to join pgrp={pgrp} for control of the \
                    terminal"
                );
                return Ok(ForegroundGuard {
                    pgrp: Some(pgrp),
                    pipeline_state: pipeline_state.clone(),
                });
            } else {
                // The state has changed, we'll have to retry
                continue;
            }
        }
    }

    /// Move the given process to the foreground.
    #[cfg(not(unix))]
    pub fn new(
        pid: u32,
        pipeline_state: &Arc<(AtomicU32, AtomicU32)>,
    ) -> std::io::Result<ForegroundGuard> {
        let _ = (pid, pipeline_state);
        Ok(ForegroundGuard {})
    }

    /// If the child process is expected to join a different process group to be in the foreground,
    /// this returns `Some(pgrp)`. This only ever returns `Some` on Unix.
    pub fn pgrp(&self) -> Option<u32> {
        #[cfg(unix)]
        {
            self.pgrp
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    /// This should only be called once by `Drop`
    fn reset_internal(&mut self) {
        #[cfg(unix)]
        {
            log::trace!("Leaving the foreground group");

            let (pgrp, pcnt) = self.pipeline_state.as_ref();
            if pcnt.fetch_sub(1, Ordering::SeqCst) == 1 {
                // Clean up if we are the last one around
                pgrp.store(0, Ordering::SeqCst);
                foreground_pgroup::reset()
            }
        }
    }
}

impl Drop for ForegroundGuard {
    fn drop(&mut self) {
        self.reset_internal();
    }
}

// It's a simpler version of fish shell's external process handling.
#[cfg(unix)]
mod foreground_pgroup {
    use nix::{
        sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
        unistd::{self, Pid},
    };
    use std::{
        os::{
            fd::{AsFd, BorrowedFd},
            unix::prelude::CommandExt,
        },
        process::{Child, Command},
    };

    /// Alternative to having to call `std::io::stdin()` just to get the file descriptor of stdin
    ///
    /// # Safety
    /// I/O safety of reading from `STDIN_FILENO` unclear.
    ///
    /// Currently only intended to access `tcsetpgrp` and `tcgetpgrp` with the I/O safe `nix`
    /// interface.
    pub unsafe fn stdin_fd() -> impl AsFd {
        unsafe { BorrowedFd::borrow_raw(nix::libc::STDIN_FILENO) }
    }

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
        let _ = unistd::tcsetpgrp(unsafe { stdin_fd() }, pgrp);
    }

    /// Reset the foreground process group to the shell
    pub fn reset() {
        if let Err(e) = unistd::tcsetpgrp(unsafe { stdin_fd() }, unistd::getpgrp()) {
            eprintln!("ERROR: reset foreground id failed, tcsetpgrp result: {e:?}");
        }
    }
}
