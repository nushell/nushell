// It's a simpler version for fish shell's external process handling.
//
// For more information, please check `child_setup_process` function in fish shell.
// https://github.com/fish-shell/fish-shell/blob/3f90efca38079922b4b21707001d7bb9630107eb/src/postfork.cpp#L140
#[cfg(target_family = "unix")]
pub mod external_process_setup {
    use std::os::unix::prelude::CommandExt;
    pub fn prepare_to_foreground(external_command: &mut std::process::Command) {
        unsafe {
            libc::signal(libc::SIGTTOU, libc::SIG_IGN);
            libc::signal(libc::SIGTTIN, libc::SIG_IGN);

            external_command.pre_exec(|| {
                // make the command startup with new process group.
                // The process group id must be the same as external commands' pid.
                // Or else we'll failed to set it as foreground process.
                // For more information, refer to `fork_child_for_process` function:
                // https://github.com/fish-shell/fish-shell/blob/023042098396aa450d2c4ea1eb9341312de23126/src/exec.cpp#L398
                libc::setpgid(0, 0);
                Ok(())
            });
        }
    }

    /// Before call this function, `prepare_to_foreground` function must be called.  Or else
    /// it'll failed with silence.
    pub fn set_foreground(process: &std::process::Child) {
        unsafe {
            libc::tcsetpgrp(libc::STDIN_FILENO, process.id() as i32);
        }
    }

    /// It can only be called when you have called `set_foreground`, or else
    /// Something strange will happened.
    pub fn reset_foreground_id() {
        unsafe {
            libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpgrp());
            libc::signal(libc::SIGTTOU, libc::SIG_DFL);
            libc::signal(libc::SIGTTIN, libc::SIG_DFL);
        }
    }
}

// TODO: investigate if we can set foreground process through windows system call.
#[cfg(target_family = "windows")]
pub mod external_process_setup {

    pub fn setup_fg_external(external_command: &mut std::process::Command) {}

    pub fn set_foreground(process: &std::process::Child) {}

    pub fn reset_foreground_id() {}
}
