#[cfg(unix)]
pub(crate) fn acquire_terminal(interactive: bool) {
    use is_terminal::IsTerminal;
    use nix::sys::signal::{signal, SigHandler, Signal};

    if !std::io::stdin().is_terminal() {
        return;
    }

    take_control(interactive);

    unsafe {
        // SIGINT and SIGQUIT have special handling above
        signal(Signal::SIGTSTP, SigHandler::SigIgn).expect("signal ignore");
        signal(Signal::SIGTTIN, SigHandler::SigIgn).expect("signal ignore");
        signal(Signal::SIGTTOU, SigHandler::SigIgn).expect("signal ignore");
    }
}

#[cfg(not(unix))]
pub(crate) fn acquire_terminal(_: bool) {}

// Inspired by fish's acquire_tty_or_exit
#[cfg(unix)]
fn take_control(interactive: bool) {
    use nix::{
        errno::Errno,
        sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal},
        unistd::{self, Pid},
    };

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

    let mut success = false;
    for _ in 0..4096 {
        match unistd::tcgetpgrp(nix::libc::STDIN_FILENO) {
            Ok(owner_pgid) if owner_pgid == shell_pgid => {
                success = true;
                break;
            }
            Ok(owner_pgid) if owner_pgid == Pid::from_raw(0) => {
                // Zero basically means something like "not owned" and we can just take it
                let _ = unistd::tcsetpgrp(nix::libc::STDIN_FILENO, shell_pgid);
            }
            Err(Errno::ENOTTY) => {
                if !interactive {
                    // that's fine
                    return;
                }
                eprintln!("ERROR: no TTY for interactive shell");
                std::process::exit(1);
            }
            _ => {
                // fish also has other heuristics than "too many attempts" for the orphan check, but they're optional
                if signal::killpg(Pid::from_raw(-shell_pgid.as_raw()), Signal::SIGTTIN).is_err() {
                    if !interactive {
                        // that's fine
                        return;
                    }
                    eprintln!("ERROR: failed to SIGTTIN ourselves");
                    std::process::exit(1);
                }
            }
        }
    }
    if !success && interactive {
        eprintln!("ERROR: failed take control of the terminal, we might be orphaned");
        std::process::exit(1);
    }
}
