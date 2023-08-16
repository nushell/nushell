#[cfg(unix)]
use std::sync::atomic::{AtomicI32, Ordering};

#[cfg(unix)]
use nix::{
    errno::Errno,
    libc,
    sys::signal::{self, signal, SaFlags, SigAction, SigHandler, SigSet, Signal},
    unistd::{self, Pid},
};

#[cfg(unix)]
use is_terminal::IsTerminal;

#[cfg(unix)]
static INITIAL_FOREGROUND_PGROUP: AtomicI32 = AtomicI32::new(-1);

#[cfg(unix)]
pub(crate) fn record_initial_foreground_process_group() {
    if let Ok(pid) = unistd::tcgetpgrp(libc::STDIN_FILENO) {
        INITIAL_FOREGROUND_PGROUP.store(pid.into(), Ordering::Relaxed);
    }
}

#[cfg(not(unix))]
pub(crate) fn record_initial_foreground_process_group() {}

#[cfg(unix)]
pub(crate) fn acquire_terminal(interactive: bool) {
    if interactive && std::io::stdin().is_terminal() {
        // see also: https://www.gnu.org/software/libc/manual/html_node/Initializing-the-Shell.html

        take_control();

        unsafe {
            // SIGINT and SIGQUIT have special handling above
            signal(Signal::SIGTSTP, SigHandler::SigIgn).expect("signal ignore");
            signal(Signal::SIGTTIN, SigHandler::SigIgn).expect("signal ignore");
            signal(Signal::SIGTTOU, SigHandler::SigIgn).expect("signal ignore");
            signal_hook::low_level::register(signal_hook::consts::SIGTERM, restore_terminal)
                .expect("signal hook");
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
        let _ = unistd::tcsetpgrp(libc::STDIN_FILENO, shell_pgid);
    }
}

#[cfg(not(unix))]
pub(crate) fn acquire_terminal(_: bool) {}

// Inspired by fish's acquire_tty_or_exit
#[cfg(unix)]
fn take_control() {
    let shell_pgid = unistd::getpgrp();

    match unistd::tcgetpgrp(nix::libc::STDIN_FILENO) {
        Ok(owner_pgid) if owner_pgid == shell_pgid => {
            // Common case, nothing to do
            return;
        }
        Ok(owner_pgid) if owner_pgid == unistd::getpid() => {
            // This can apparently happen with sudo: https://github.com/fish-shell/fish-shell/issues/7388
            let _ = unistd::setpgid(owner_pgid, owner_pgid);
            return;
        }
        _ => (),
    }

    // Reset all signal handlers to default
    for sig in Signal::iterator() {
        unsafe {
            if let Ok(old_act) = signal::sigaction(
                sig,
                &SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty()),
            ) {
                // fish preserves ignored SIGHUP, presumably for nohup support, so let's do the same
                if sig == Signal::SIGHUP && old_act.handler() == SigHandler::SigIgn {
                    let _ = signal::sigaction(sig, &old_act);
                }
            }
        }
    }

    for _ in 0..4096 {
        match unistd::tcgetpgrp(libc::STDIN_FILENO) {
            Ok(owner_pgid) if owner_pgid == shell_pgid => {
                // success
                return;
            }
            Ok(owner_pgid) if owner_pgid == Pid::from_raw(0) => {
                // Zero basically means something like "not owned" and we can just take it
                let _ = unistd::tcsetpgrp(libc::STDIN_FILENO, shell_pgid);
            }
            Err(Errno::ENOTTY) => {
                eprintln!("ERROR: no TTY for interactive shell");
                std::process::exit(1);
            }
            _ => {
                // fish also has other heuristics than "too many attempts" for the orphan check, but they're optional
                if signal::killpg(shell_pgid, Signal::SIGTTIN).is_err() {
                    eprintln!("ERROR: failed to SIGTTIN ourselves");
                    std::process::exit(1);
                }
            }
        }
    }

    eprintln!("ERROR: failed take control of the terminal, we might be orphaned");
    std::process::exit(1);
}

#[cfg(unix)]
pub(crate) fn restore_terminal() {
    // Safety: can only call async-signal-safe functions here
    // tcsetpgrp and getpgrp are async-signal-safe
    let initial_pgid = Pid::from_raw(INITIAL_FOREGROUND_PGROUP.load(Ordering::Relaxed));
    if initial_pgid.as_raw() > 0 && initial_pgid != unistd::getpgrp() {
        let _ = unistd::tcsetpgrp(libc::STDIN_FILENO, initial_pgid);
    }
}

#[cfg(not(unix))]
pub(crate) fn restore_terminal() {}
