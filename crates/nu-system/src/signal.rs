use nix::sys::signal::{self, SigHandler, Signal};

/// Blocks the SIGTSTP/SIGTTOU/SIGTTIN/SIGCHLD signals so that the shell never receives
/// them.
pub fn block() {
    let mut sigset = signal::SigSet::empty();
    sigset.add(signal::Signal::SIGTSTP);
    sigset.add(signal::Signal::SIGTTOU);
    sigset.add(signal::Signal::SIGTTIN);
    sigset.add(signal::Signal::SIGCHLD);
    signal::sigprocmask(signal::SigmaskHow::SIG_BLOCK, Some(&sigset), None)
        .expect("Could not block the signals");
}

/// Unblocks the SIGTSTP/SIGTTOU/SIGTTIN/SIGCHLD signals so children processes can be
/// controlled
/// by the shell.
pub fn unblock() {
    let mut sigset = signal::SigSet::empty();
    sigset.add(signal::Signal::SIGTSTP);
    sigset.add(signal::Signal::SIGTTOU);
    sigset.add(signal::Signal::SIGTTIN);
    sigset.add(signal::Signal::SIGCHLD);
    signal::sigprocmask(signal::SigmaskHow::SIG_UNBLOCK, Some(&sigset), None)
        .expect("Could not block the signals");
}

pub fn set_terminal_leader() {
    let stdin_is_a_tty = atty::is(atty::Stream::Stdin);
    if stdin_is_a_tty {
        // We have make sure that stdin is a tty, it's ok to ignore SIGTTOU.
        unsafe {
            if let Err(e) = signal::signal(Signal::SIGTTOU, SigHandler::SigIgn) {
                println!("WARN: ignore SIGTTOU failed, error message: {e:?}");
            }
        }
    }
}
