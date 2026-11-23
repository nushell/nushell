use nix::{
    errno::Errno,
    fcntl::{FcntlArg::F_SETFD, FdFlag, fcntl},
    libc,
    pty::openpty,
    unistd::setsid,
};
use nu_test_support::prelude::*;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    os::{fd::OwnedFd, unix::net::UnixStream, unix::process::CommandExt},
    process::Command,
    time::Duration,
};

/// Give interactive Nu its own controlling terminal. The pipeline prints "ready" once it
/// can accept keystrokes. Capture output separately so terminal echo cannot satisfy assertions.
fn run_pipeline(code: &str, keys: &[u8]) -> io::Result<Vec<String>> {
    let pty = openpty(None, None)?;
    for fd in [&pty.master, &pty.slave] {
        fcntl(fd, F_SETFD(FdFlag::FD_CLOEXEC))?;
    }
    let mut terminal = File::from(pty.master);
    let (output, writer) = UnixStream::pair()?;
    output.set_read_timeout(Some(Duration::from_secs(15)))?;
    let mut nu = Command::new(NU.path());
    nu.args(["-n", "-i", "-c", code])
        .stdin(pty.slave)
        .stdout(OwnedFd::from(writer.try_clone()?))
        .stderr(OwnedFd::from(writer));
    // SAFETY: only set up the child's session and controlling terminal before exec.
    unsafe {
        nu.pre_exec(|| {
            setsid()?;
            Errno::result(libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY as _, 0))?;
            Ok(())
        });
    }
    let mut child = nu.spawn()?;
    drop(nu); // Close our copy of the output socket's write end.

    let mut lines = BufReader::new(output).lines();
    let result = (|| {
        let ready = lines.next().transpose()?;
        if ready.as_deref() != Some("ready") {
            return Err(io::Error::other(format!("expected ready, got {ready:?}")));
        }
        terminal.write_all(keys)?;
        let mut read = Vec::new();
        for line in lines {
            match line {
                Ok(line) => read.push(line),
                Err(err) => {
                    return Err(io::Error::new(
                        err.kind(),
                        format!("{err}, read so far: {read:?}"),
                    ));
                }
            }
        }
        Ok(read)
    })();
    drop(terminal);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait()?;
    let lines = result?;
    assert!(status.success(), "nu exited with {status}: {lines:?}");
    Ok(lines)
}

#[test]
#[deps(NU)]
fn ctrl_c_interrupts_both_external_commands() -> Result {
    // Forward readiness through the pipe so both INT traps are installed before Ctrl+C.
    let code = r#"
        ^sh -c 'trap "echo producer >&2; exit 0" INT; echo ready; read -r line'
        | ^sh -c 'trap "echo consumer; exit 0" INT; read -r line && echo "$line"; read -r line'
        print resumed
    "#;
    let mut lines = run_pipeline(code, b"\x03")?;
    // Nu moves on only after both commands exit, but they can print in either order.
    assert_eq!(lines.pop().as_deref(), Some("resumed"), "{lines:?}");
    lines.sort_unstable();
    assert_eq!(lines, ["consumer", "producer"]);
    Ok(())
}
