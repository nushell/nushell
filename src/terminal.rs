use std::{
    io::IsTerminal,
    sync::atomic::{AtomicI32, Ordering},
};

use nix::{
    errno::Errno,
    libc,
    sys::signal::{killpg, raise, sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
    unistd::{self, Pid},
};

static INITIAL_PGID: AtomicI32 = AtomicI32::new(-1);

pub(crate) fn acquire(interactive: bool) {
    if interactive && std::io::stdin().is_terminal() {
        // see also: https://www.gnu.org/software/libc/manual/html_node/Initializing-the-Shell.html

        if unsafe { libc::atexit(restore_terminal) } != 0 {
            eprintln!("ERROR: failed to set exit function");
            std::process::exit(1);
        };

        let initial_pgid = take_control();

        INITIAL_PGID.store(initial_pgid.into(), Ordering::Relaxed);

        unsafe {
            // SIGINT has special handling
            let ignore = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
            sigaction(Signal::SIGQUIT, &ignore).expect("signal ignore");
            sigaction(Signal::SIGTSTP, &ignore).expect("signal ignore");
            sigaction(Signal::SIGTTIN, &ignore).expect("signal ignore");
            sigaction(Signal::SIGTTOU, &ignore).expect("signal ignore");
            sigaction(
                Signal::SIGTERM,
                &SigAction::new(
                    SigHandler::Handler(sigterm_handler),
                    SaFlags::empty(),
                    SigSet::empty(),
                ),
            )
            .expect("signal action");
        }

        // Put ourselves in our own process group, if not already
        let shell_pgid = unistd::getpid();
        match unistd::setpgid(shell_pgid, shell_pgid) {
            // setpgid returns EPERM if we are the session leader (e.g., as a login shell).
            // The other cases that return EPERM cannot happen, since we gave our own pid.
            // See: setpgid(2)
            // Therefore, it is safe to ignore EPERM.
            Ok(()) | Err(Errno::EPERM) => (),
            Err(_) => {
                eprintln!("ERROR: failed to put nushell in its own process group");
                std::process::exit(1);
            }
        }
        // Set our possibly new pgid to be in control of terminal
        let _ = unistd::tcsetpgrp(unsafe { nu_system::stdin_fd() }, shell_pgid);
    }
}

// Inspired by fish's acquire_tty_or_exit
// Returns our original pgid
fn take_control() -> Pid {
    let shell_pgid = unistd::getpgrp();

    match unistd::tcgetpgrp(unsafe { nu_system::stdin_fd() }) {
        Ok(owner_pgid) if owner_pgid == shell_pgid => {
            // Common case, nothing to do
            return owner_pgid;
        }
        Ok(owner_pgid) if owner_pgid == unistd::getpid() => {
            // This can apparently happen with sudo: https://github.com/fish-shell/fish-shell/issues/7388
            let _ = unistd::setpgid(owner_pgid, owner_pgid);
            return owner_pgid;
        }
        _ => (),
    }

    // Reset all signal handlers to default
    let default = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
    for sig in Signal::iterator() {
        if let Ok(old_act) = unsafe { sigaction(sig, &default) } {
            // fish preserves ignored SIGHUP, presumably for nohup support, so let's do the same
            if sig == Signal::SIGHUP && old_act.handler() == SigHandler::SigIgn {
                let _ = unsafe { sigaction(sig, &old_act) };
            }
        }
    }

    for _ in 0..4096 {
        match unistd::tcgetpgrp(unsafe { nu_system::stdin_fd() }) {
            Ok(owner_pgid) if owner_pgid == shell_pgid => {
                // success
                return owner_pgid;
            }
            Ok(owner_pgid) if owner_pgid == Pid::from_raw(0) => {
                // Zero basically means something like "not owned" and we can just take it
                let _ = unistd::tcsetpgrp(unsafe { nu_system::stdin_fd() }, shell_pgid);
            }
            Err(Errno::ENOTTY) => {
                eprintln!("ERROR: no TTY for interactive shell");
                std::process::exit(1);
            }
            _ => {
                // fish also has other heuristics than "too many attempts" for the orphan check, but they're optional
                if killpg(shell_pgid, Signal::SIGTTIN).is_err() {
                    eprintln!("ERROR: failed to SIGTTIN ourselves");
                    std::process::exit(1);
                }
            }
        }
    }

    eprintln!("ERROR: failed to take control of the terminal, we might be orphaned");
    std::process::exit(1);
}

extern "C" fn restore_terminal() {
    // Safety: can only call async-signal-safe functions here
    // `tcsetpgrp` and `getpgrp` are async-signal-safe
    let initial_pgid = Pid::from_raw(INITIAL_PGID.load(Ordering::Relaxed));
    if initial_pgid.as_raw() > 0 && initial_pgid != unistd::getpgrp() {
        let _ = unistd::tcsetpgrp(unsafe { nu_system::stdin_fd() }, initial_pgid);
    }
}

extern "C" fn sigterm_handler(_signum: libc::c_int) {
    // Safety: can only call async-signal-safe functions here
    // `restore_terminal`, `sigaction`, `raise`, and `_exit` are all async-signal-safe

    restore_terminal();

    let default = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
    if unsafe { sigaction(Signal::SIGTERM, &default) }.is_err() {
        // Failed to set signal handler to default.
        // This should not be possible, but if it does happen,
        // then this could result in an infinite loop due to the raise below.
        // So, we'll just exit immediately if this happens.
        unsafe { libc::_exit(1) };
    };

    if raise(Signal::SIGTERM).is_err() {
        unsafe { libc::_exit(1) };
    };
}
