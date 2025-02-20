use std::io;
use std::process::Command as CommandSys;

/// Tries to forcefully kill a process by its PID
pub fn kill_by_pid(pid: i64) -> io::Result<()> {
    let mut cmd = build_kill_command(true, std::iter::once(pid), None);

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "failed to kill process",
        ));
    }

    Ok(())
}

/// Create a `std::process::Command` for the current target platform, for killing
/// the processes with the given PIDs
pub fn build_kill_command(
    force: bool,
    pids: impl Iterator<Item = i64>,
    signal: Option<u32>,
) -> CommandSys {
    if cfg!(windows) {
        let mut cmd = CommandSys::new("taskkill");

        if force {
            cmd.arg("/F");
        }

        // each pid must written as `/PID 0` otherwise
        // taskkill will act as `killall` unix command
        for id in pids {
            cmd.arg("/PID");
            cmd.arg(id.to_string());
        }

        cmd
    } else {
        let mut cmd = CommandSys::new("kill");
        if let Some(signal_value) = signal {
            cmd.arg(format!("-{}", signal_value));
        } else if force {
            cmd.arg("-9");
        }

        cmd.args(pids.map(move |id| id.to_string()));

        cmd
    }
}
