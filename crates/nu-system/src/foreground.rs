use std::process::{Child, Command};

/// A simple wrapper for `std::process::Command`
///
/// ## spawn behavior
/// ### Unix
/// When invoke `spawn`, current process will ignore `SIGTTOU`, `SIGTTIN` signal, spawned child process will get it's
/// own process group id, and it's going to foreground(by making stdin belong's to child's process group).
///
/// When child is to over, `SIGTTOU`, `SIGTTIN` signal will be reset, foreground process is back to callers' process.
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
        self.inner.spawn().map(|child| {
            fg_process_setup::set_foreground(&child);
            ForegroundChild { inner: child }
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
        // It's ok to use here because we have called `set_foreground` during creation.
        unsafe { fg_process_setup::reset_foreground_id() }
    }
}

// It's a simpler version of fish shell's external process handling.
//
// For more information, please check `child_setup_process` function in fish shell.
// https://github.com/fish-shell/fish-shell/blob/3f90efca38079922b4b21707001d7bb9630107eb/src/postfork.cpp#L140
#[cfg(target_family = "unix")]
mod fg_process_setup {
    use std::os::unix::prelude::CommandExt;
    pub fn prepare_to_foreground(external_command: &mut std::process::Command) {
        unsafe {
            libc::signal(libc::SIGTTOU, libc::SIG_IGN);
            libc::signal(libc::SIGTTIN, libc::SIG_IGN);

            // Safety:
            // POSIX only allows async-signal-safe functions to be called.
            // And `setpgid` is async-signal-safe function according to:
            // https://manpages.ubuntu.com/manpages/bionic/man7/signal-safety.7.html
            // So we're ok to invoke `libc::setpgid` inside `pre_exec`.
            external_command.pre_exec(|| {
                // make the command startup with new process group.
                // The process group id must be the same as external commands' pid.
                // Or else we'll failed to set it as foreground process.
                // For more information, check `fork_child_for_process` function:
                // https://github.com/fish-shell/fish-shell/blob/023042098396aa450d2c4ea1eb9341312de23126/src/exec.cpp#L398
                libc::setpgid(0, 0);
                Ok(())
            });
        }
    }

    // If `prepare_to_foreground` function is not called, the function will fail with silence and do nothing.
    pub fn set_foreground(process: &std::process::Child) {
        // it's ok to use unsafe here
        // the implementaion here is just the same as
        // https://docs.rs/nix/latest/nix/unistd/fn.tcsetpgrp.html, which is a safe function.
        unsafe {
            libc::tcsetpgrp(libc::STDIN_FILENO, process.id() as i32);
        }
    }

    /// Reset foreground to current process, and reset back `SIGTTOU`, `SIGTTIN` single handler.
    ///
    /// ## Safety
    /// It can only be called when you have called `set_foreground`, or results in undefined behavior.
    pub unsafe fn reset_foreground_id() {
        libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpgrp());
        libc::signal(libc::SIGTTOU, libc::SIG_DFL);
        libc::signal(libc::SIGTTIN, libc::SIG_DFL);
    }
}

// TODO: investigate if we can set foreground process through windows system call.
#[cfg(target_family = "windows")]
mod fg_process_setup {

    pub fn prepare_to_foreground(_external_command: &mut std::process::Command) {}

    pub fn set_foreground(_process: &std::process::Child) {}

    pub unsafe fn reset_foreground_id() {}
}
