use std::{
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
/// The spawned child process will get its own process group id, and it's going to foreground (by making stdin belong's to child's process group).
///
/// On drop, the calling process's group will become the foreground process group once again.
///
/// ### Windows
/// It does nothing special on windows system, `spawn` is the same as [std::process::Command::spawn](std::process::Command::spawn)
pub struct ForegroundProcess {
    inner: Command,
    pipeline_state: Arc<(AtomicU32, AtomicU32)>,
}

/// A simple wrapper for `std::process::Child`
///
/// It can only be created by `ForegroundProcess::spawn`.
pub struct ForegroundChild {
    inner: Child,
    pipeline_state: Arc<(AtomicU32, AtomicU32)>,
}

impl ForegroundProcess {
    pub fn new(cmd: Command, pipeline_state: Arc<(AtomicU32, AtomicU32)>) -> Self {
        Self {
            inner: cmd,
            pipeline_state,
        }
    }

    pub fn spawn(&mut self) -> std::io::Result<ForegroundChild> {
        let (ref pgrp, ref pcnt) = *self.pipeline_state;
        let existing_pgrp = pgrp.load(Ordering::SeqCst);
        fg_process_setup::prepare_to_foreground(&mut self.inner, existing_pgrp);
        self.inner
            .spawn()
            .map(|child| {
                fg_process_setup::set_foreground(&child, existing_pgrp);
                let _ = pcnt.fetch_add(1, Ordering::SeqCst);
                if existing_pgrp == 0 {
                    pgrp.store(child.id(), Ordering::SeqCst);
                }
                ForegroundChild {
                    inner: child,
                    pipeline_state: self.pipeline_state.clone(),
                }
            })
            .map_err(|e| {
                fg_process_setup::reset_foreground_id();
                e
            })
    }
}

impl AsMut<Child> for ForegroundChild {
    fn as_mut(&mut self) -> &mut Child {
        &mut self.inner
    }
}

impl Drop for ForegroundChild {
    fn drop(&mut self) {
        let (ref pgrp, ref pcnt) = *self.pipeline_state;
        if pcnt.fetch_sub(1, Ordering::SeqCst) == 1 {
            pgrp.store(0, Ordering::SeqCst);
            fg_process_setup::reset_foreground_id()
        }
    }
}

// It's a simpler version of fish shell's external process handling.
// Note: we exclude macos because the techniques below seem to have issues in macos 13 currently.
#[cfg(all(target_family = "unix", not(target_os = "macos")))]
mod fg_process_setup {
    use nix::{
        sys::signal,
        unistd::{self, Pid},
    };
    use std::os::unix::prelude::{CommandExt, RawFd};

    // TODO: when raising MSRV past 1.63.0, switch to OwnedFd
    struct TtyHandle(RawFd);

    impl Drop for TtyHandle {
        fn drop(&mut self) {
            let _ = unistd::close(self.0);
        }
    }

    pub(super) fn prepare_to_foreground(
        external_command: &mut std::process::Command,
        existing_pgrp: u32,
    ) {
        let tty = TtyHandle(unistd::dup(nix::libc::STDIN_FILENO).expect("dup"));
        unsafe {
            // Safety:
            // POSIX only allows async-signal-safe functions to be called.
            // `sigprocmask`, `setpgid` and `tcsetpgrp` are async-signal-safe according to:
            // https://manpages.ubuntu.com/manpages/bionic/man7/signal-safety.7.html
            external_command.pre_exec(move || {
                // When this callback is run, std::process has already done:
                // - pthread_sigmask(SIG_SETMASK) with an empty sigset
                // - signal(SIGPIPE, SIG_DFL)
                // However, we do need TTOU/TTIN blocked again during this setup.
                let mut sigset = signal::SigSet::empty();
                sigset.add(signal::Signal::SIGTSTP);
                sigset.add(signal::Signal::SIGTTOU);
                sigset.add(signal::Signal::SIGTTIN);
                sigset.add(signal::Signal::SIGCHLD);
                signal::sigprocmask(signal::SigmaskHow::SIG_BLOCK, Some(&sigset), None)
                    .expect("signal mask");

                // According to glibc's job control manual:
                // https://www.gnu.org/software/libc/manual/html_node/Launching-Jobs.html
                // This has to be done *both* in the parent and here in the child due to race conditions.
                set_foreground_pid(unistd::getpid(), existing_pgrp, tty.0);

                // Now let the child process have all the signals by resetting with SIG_SETMASK.
                let mut sigset = signal::SigSet::empty();
                sigset.add(signal::Signal::SIGTSTP); // for now not really all: we don't support background jobs, so keep this one blocked
                signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None)
                    .expect("signal mask");

                Ok(())
            });
        }
    }

    pub(super) fn set_foreground(process: &std::process::Child, existing_pgrp: u32) {
        // called from the parent shell process - do the stdin tty check here
        if atty::is(atty::Stream::Stdin) {
            set_foreground_pid(
                Pid::from_raw(process.id() as i32),
                existing_pgrp,
                nix::libc::STDIN_FILENO,
            );
        }
    }

    // existing_pgrp is 0 when we don't have an existing foreground process in the pipeline.
    // Conveniently, 0 means "current pid" to setpgid. But not to tcsetpgrp.
    fn set_foreground_pid(pid: Pid, existing_pgrp: u32, tty: RawFd) {
        let _ = unistd::setpgid(pid, Pid::from_raw(existing_pgrp as i32));
        let _ = unistd::tcsetpgrp(
            tty,
            if existing_pgrp == 0 {
                pid
            } else {
                Pid::from_raw(existing_pgrp as i32)
            },
        );
    }

    /// Reset the foreground process group to the shell
    pub(super) fn reset_foreground_id() {
        if atty::is(atty::Stream::Stdin) {
            if let Err(e) = nix::unistd::tcsetpgrp(nix::libc::STDIN_FILENO, unistd::getpgrp()) {
                println!("ERROR: reset foreground id failed, tcsetpgrp result: {e:?}");
            }
        }
    }
}

#[cfg(any(not(target_family = "unix"), target_os = "macos"))]
mod fg_process_setup {
    pub(super) fn prepare_to_foreground(_: &mut std::process::Command, _: u32) {}

    pub(super) fn set_foreground(_: &std::process::Child, _: u32) {}

    pub(super) fn reset_foreground_id() {}
}
