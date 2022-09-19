use std::process::{Child, Command};

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
}

/// A simple wrapper for `std::process::Child`
///
/// It can only be created by `ForegroundProcess::spawn`.
pub struct ForegroundChild {
    inner: Child,
}

impl ForegroundProcess {
    pub fn new(cmd: Command) -> Self {
        Self { inner: cmd }
    }

    pub fn spawn(&mut self) -> std::io::Result<ForegroundChild> {
        fg_process_setup::prepare_to_foreground(&mut self.inner);
        self.inner
            .spawn()
            .map(|child| {
                fg_process_setup::set_foreground(&child);
                ForegroundChild { inner: child }
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
        fg_process_setup::reset_foreground_id()
    }
}

// It's a simpler version of fish shell's external process handling.
#[cfg(target_family = "unix")]
mod fg_process_setup {
    use nix::{
        sys::signal,
        unistd::{self, Pid},
    };
    use std::os::unix::prelude::CommandExt;

    pub(super) fn prepare_to_foreground(external_command: &mut std::process::Command) {
        unsafe {
            // Safety:
            // POSIX only allows async-signal-safe functions to be called.
            // `sigprocmask`, `setpgid` and `tcsetpgrp` are async-signal-safe according to:
            // https://manpages.ubuntu.com/manpages/bionic/man7/signal-safety.7.html
            external_command.pre_exec(|| {
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
                let _ = set_foreground_pid(unistd::getpid());

                // Now let the child process have all the signals by resetting with SIG_SETMASK.
                let mut sigset = signal::SigSet::empty();
                sigset.add(signal::Signal::SIGTSTP); // for now not really all: we don't support background jobs, so keep this one blocked
                signal::sigprocmask(signal::SigmaskHow::SIG_SETMASK, Some(&sigset), None)
                    .expect("signal mask");

                Ok(())
            });
        }
    }

    pub(super) fn set_foreground(process: &std::process::Child) {
        let _ = set_foreground_pid(Pid::from_raw(process.id() as i32));
    }

    fn set_foreground_pid(pid: Pid) -> nix::Result<()> {
        if atty::is(atty::Stream::Stdin) {
            unistd::setpgid(pid, pid)?;
            unistd::tcsetpgrp(nix::libc::STDIN_FILENO, pid)?;
        }
        Ok(())
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

#[cfg(not(target_family = "unix"))]
mod fg_process_setup {
    pub(super) fn prepare_to_foreground(_external_command: &mut std::process::Command) {}

    pub(super) fn set_foreground(_process: &std::process::Child) {}

    pub(super) fn reset_foreground_id() {}
}
