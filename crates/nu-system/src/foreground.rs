// It's a simpler version for fish shell's external process handling.
//
// For more information, please check `child_setup_process` function in fish shell.
// https://github.com/fish-shell/fish-shell/blob/3f90efca38079922b4b21707001d7bb9630107eb/src/postfork.cpp#L140
#[cfg(target_family = "unix")]
pub mod external_process_setup {
    use std::os::unix::prelude::CommandExt;
    pub fn setup_fg_external(external_command: &mut std::process::Command) {
        unsafe {
            libc::signal(libc::SIGTTOU, libc::SIG_IGN);
            libc::signal(libc::SIGTTIN, libc::SIG_IGN);

            external_command.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }
    }

    pub fn set_foreground(process: &std::process::Child) -> i32 {
        unsafe {
            let my_id = libc::getpid();
            libc::tcsetpgrp(0, process.id() as i32);
            my_id
        }
    }

    pub fn reset_foreground_id() {
        unsafe {
            libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpgrp());
            libc::signal(libc::SIGTTOU, libc::SIG_DFL);
            libc::signal(libc::SIGTTIN, libc::SIG_DFL);
        }
    }
}

#[cfg(target_family = "windows")]
mod external_process_setup {

    pub fn setup_fg_external(external_command: &mut std::process::Command) {}

    pub fn set_foreground(process: &std::process::Child) -> i32 {}

    pub fn reset_foreground_id() {}
}
