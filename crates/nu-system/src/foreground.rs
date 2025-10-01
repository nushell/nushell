use std::sync::{Arc, atomic::AtomicU32};

use std::io;

use std::process::{Child, Command};

use crate::ExitStatus;

#[cfg(unix)]
use std::{io::IsTerminal, sync::atomic::Ordering};

#[cfg(unix)]
pub use child_pgroup::stdin_fd;

#[cfg(unix)]
use nix::{sys::signal, sys::wait, unistd::Pid};

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

    // this is unix-only since we don't have to deal with process groups in windows
    #[cfg(unix)]
    interactive: bool,
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
        background: bool,
        pipeline_state: &Arc<(AtomicU32, AtomicU32)>,
    ) -> io::Result<Self> {
        let interactive = interactive && io::stdin().is_terminal();

        let uses_dedicated_process_group = interactive || background;

        if uses_dedicated_process_group {
            let (pgrp, pcnt) = pipeline_state.as_ref();
            let existing_pgrp = pgrp.load(Ordering::SeqCst);
            child_pgroup::prepare_command(&mut command, existing_pgrp, background);
            command
                .spawn()
                .map(|child| {
                    child_pgroup::set(&child, existing_pgrp, background);

                    let _ = pcnt.fetch_add(1, Ordering::SeqCst);
                    if existing_pgrp == 0 {
                        pgrp.store(child.id(), Ordering::SeqCst);
                    }
                    Self {
                        inner: child,
                        pipeline_state: Some(pipeline_state.clone()),
                        interactive,
                    }
                })
                .inspect_err(|_e| {
                    if interactive {
                        child_pgroup::reset();
                    }
                })
        } else {
            command.spawn().map(|child| Self {
                inner: child,
                pipeline_state: None,
                interactive,
            })
        }
    }

    pub fn wait(&mut self) -> io::Result<ForegroundWaitStatus> {
        #[cfg(unix)]
        {
            let child_pid = Pid::from_raw(self.inner.id() as i32);

            unix_wait(child_pid).inspect(|result| {
                if let (true, ForegroundWaitStatus::Frozen(_)) = (self.interactive, result) {
                    child_pgroup::reset();
                }
            })
        }
        #[cfg(not(unix))]
        self.as_mut().wait().map(Into::into)
    }

    pub fn pid(&self) -> u32 {
        self.inner.id()
    }
}

#[cfg(unix)]
fn unix_wait(child_pid: Pid) -> std::io::Result<ForegroundWaitStatus> {
    use ForegroundWaitStatus::*;

    // the child may be stopped multiple times, we loop until it exits
    loop {
        let status = wait::waitpid(child_pid, Some(wait::WaitPidFlag::WUNTRACED));
        match status {
            Err(e) => {
                return Err(e.into());
            }
            Ok(wait::WaitStatus::Exited(_, status)) => {
                return Ok(Finished(ExitStatus::Exited(status)));
            }
            Ok(wait::WaitStatus::Signaled(_, signal, core_dumped)) => {
                return Ok(Finished(ExitStatus::Signaled {
                    signal: signal as i32,
                    core_dumped,
                }));
            }
            Ok(wait::WaitStatus::Stopped(_, _)) => {
                return Ok(Frozen(UnfreezeHandle { child_pid }));
            }
            Ok(_) => {
                // keep waiting
            }
        };
    }
}

pub enum ForegroundWaitStatus {
    Finished(ExitStatus),
    Frozen(UnfreezeHandle),
}

impl From<std::process::ExitStatus> for ForegroundWaitStatus {
    fn from(status: std::process::ExitStatus) -> Self {
        ForegroundWaitStatus::Finished(status.into())
    }
}

#[derive(Debug)]
pub struct UnfreezeHandle {
    #[cfg(unix)]
    child_pid: Pid,
}

impl UnfreezeHandle {
    #[cfg(unix)]
    pub fn unfreeze(
        self,
        pipeline_state: Option<Arc<(AtomicU32, AtomicU32)>>,
    ) -> io::Result<ForegroundWaitStatus> {
        // bring child's process group back into foreground and continue it

        // we only keep the guard for its drop impl
        let _guard = pipeline_state.map(|pipeline_state| {
            ForegroundGuard::new(self.child_pid.as_raw() as u32, &pipeline_state)
        });

        if let Err(err) = signal::killpg(self.child_pid, signal::SIGCONT) {
            return Err(err.into());
        }

        let child_pid = self.child_pid;

        unix_wait(child_pid)
    }

    pub fn pid(&self) -> u32 {
        #[cfg(unix)]
        {
            self.child_pid.as_raw() as u32
        }

        #[cfg(not(unix))]
        0
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
        if let Some((pgrp, pcnt)) = self.pipeline_state.as_deref()
            && pcnt.fetch_sub(1, Ordering::SeqCst) == 1
        {
            pgrp.store(0, Ordering::SeqCst);

            if self.interactive {
                child_pgroup::reset()
            }
        }
    }
}

/// Keeps a specific already existing process in the foreground as long as the [`ForegroundGuard`].
/// If the process needs to be spawned in the foreground, use [`ForegroundChild`] instead. This is
/// used to temporarily bring frozen and plugin processes into the foreground.
///
/// # OS-specific behavior
/// ## Unix
///
/// If there is already a foreground external process running, spawned with [`ForegroundChild`],
/// this expects the process ID to remain in the process group created by the [`ForegroundChild`]
/// for the lifetime of the guard, and keeps the terminal controlling process group set to that.
/// If there is no foreground external process running, this sets the foreground process group to
/// the provided process ID. The process group that is expected can be retrieved with
/// [`.pgrp()`](Self::pgrp) if different from the provided process ID.
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

                log::trace!("Giving control of the terminal to the process group, pid={pid}");

                // Set the terminal controlling process group to the child process
                unistd::tcsetpgrp(unsafe { stdin_fd() }, pid_nix)?;

                return Ok(guard);
            } else if pcnt
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                    // Avoid a race condition: only increment if count is > 0
                    if count > 0 { Some(count + 1) } else { None }
                })
                .is_ok()
            {
                // We successfully added another count to the foreground process group, which means
                // we only need to tell the child process to join this one
                let pgrp = pgrp.load(Ordering::SeqCst);
                log::trace!(
                    "Will ask the process pid={pid} to join pgrp={pgrp} for control of the \
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
                child_pgroup::reset()
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
mod child_pgroup {
    use nix::{
        sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction},
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

    pub fn prepare_command(external_command: &mut Command, existing_pgrp: u32, background: bool) {
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
                set_foreground_pid(Pid::this(), existing_pgrp, background);

                // `terminal.rs` makes the shell process ignore some signals,
                //  so we set them to their default behavior for our child
                let default = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());

                let _ = sigaction(Signal::SIGQUIT, &default);
                let _ = sigaction(Signal::SIGTSTP, &default);
                let _ = sigaction(Signal::SIGTERM, &default);

                Ok(())
            });
        }
    }

    pub fn set(process: &Child, existing_pgrp: u32, background: bool) {
        set_foreground_pid(
            Pid::from_raw(process.id() as i32),
            existing_pgrp,
            background,
        );
    }

    fn set_foreground_pid(pid: Pid, existing_pgrp: u32, background: bool) {
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

        if !background {
            let _ = unistd::tcsetpgrp(unsafe { stdin_fd() }, pgrp);
        }
    }

    /// Reset the foreground process group to the shell
    pub fn reset() {
        if let Err(e) = unistd::tcsetpgrp(unsafe { stdin_fd() }, unistd::getpgrp()) {
            eprintln!("ERROR: reset foreground id failed, tcsetpgrp result: {e:?}");
        }
    }
}
