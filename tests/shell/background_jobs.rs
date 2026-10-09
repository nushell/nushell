//! How background jobs end with Nu, checked against real `nu` processes.
//!
//! Nu is interrupted with `kill --signal 2 $nu.pid` and runs Unix commands, so these tests only run
//! on Unix.

use nix::libc;
use nu_test_support::{fs::Stub::FileWithContent, prelude::*};
use nu_utils::time::Instant;
use pretty_assertions::assert_eq;
use rstest::rstest;
use std::{
    io::{self, Read},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::Path,
    process::{Child, Command, Output, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

/// How long to wait for what should happen right away. Only failing tests wait this long, so it's
/// generous enough to hold up on loaded CI machines.
const TIMEOUT: Duration = Duration::from_secs(30);

/// How long `nu` may run before it's killed: longer than any wait in the code it runs.
const NU_TIMEOUT: Duration = Duration::from_secs(180);

/// Starts a background job running an external process, and waits until the job has registered
/// the process, so that killing the job kills it.
///
/// The process ends by itself after two minutes: long enough to still run when a test checks for
/// it, and short enough not to linger long after a test that leaks it.
const SPAWN_BACKGROUND_PROCESS: &str = "
    job spawn { ^sleep 120 } | ignore
    while (job list | get pids | flatten | is-empty) { sleep 10ms }
    job list | get pids | flatten | first | save background.pid
";

/// Interrupts Nu and catches the interrupt, which leaves the code running.
const CATCH_INTERRUPT: &str = "
    try { kill --signal 2 $nu.pid; sleep 1min } catch {|err| $err.msg | save interrupt.txt }
";

/// A `nu` command running in `dir` without a terminal, user configuration or standard library.
fn nu(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(NU.path());
    command
        .current_dir(dir)
        .arg("--config-home")
        .arg(dir)
        .arg("--no-std-lib")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// Wait for `nu` to exit, killing it if it takes longer than [`NU_TIMEOUT`].
fn wait_for(mut child: Child) -> Result<Output> {
    // Read the output while `nu` runs, so that it can't block on a full pipe.
    let stdout = read_in_background(child.stdout.take());
    let stderr = read_in_background(child.stderr.take());
    let deadline = Instant::now() + NU_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            // `nu` hasn't been reaped, so `kill` can't hit another process that reused its PID.
            let _ = child.kill();
            let _ = child.wait();
            panic!("nu didn't exit within {NU_TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    let output = |receiver: mpsc::Receiver<io::Result<Vec<u8>>>| {
        receiver
            .recv_timeout(TIMEOUT)
            .expect("a process that nu started kept its output open")
    };
    Ok(Output {
        status,
        stdout: output(stdout)?,
        stderr: output(stderr)?,
    })
}

/// Read `pipe` to the end on another thread.
fn read_in_background(
    pipe: Option<impl Read + Send + 'static>,
) -> mpsc::Receiver<io::Result<Vec<u8>>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = match pipe {
            Some(mut pipe) => pipe.read_to_end(&mut bytes).map(|_| bytes),
            None => Ok(bytes),
        };
        let _ = sender.send(result);
    });
    receiver
}

/// Run `nu` and check that no process it started outlives it.
///
/// Every process that `nu` starts inherits the write end of a pipe, so the pipe closes once they
/// and `nu` have all exited. Unlike probing a PID, this isn't fooled by an unreaped zombie, which
/// has closed its files, or by a PID that was reused.
///
/// A process that outlives `nu` is left to end by itself. The open pipe only shows that some
/// process `nu` started still holds it, not which one, so the recorded PID may already belong to
/// an unrelated process.
fn assert_nothing_outlives_nu(dir: &Path, args: &[&str], exit_code: i32) -> Result {
    let (mut reader, writer) = io::pipe()?;
    let fd = writer.as_raw_fd();
    let mut command = nu(dir, args);
    // SAFETY: `fcntl` is async-signal-safe.
    unsafe {
        command.pre_exec(move || {
            // Unlike the test's other processes, `nu` inherits the write end.
            match libc::fcntl(fd, libc::F_SETFD, 0) {
                -1 => Err(io::Error::last_os_error()),
                _ => Ok(()),
            }
        });
    }
    let child = command.spawn()?;
    drop(writer);
    let output = wait_for(child)?;

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(reader.read_to_end(&mut Vec::new())));
    let closed = receiver.recv_timeout(TIMEOUT).is_ok();

    assert!(
        read(dir, "background.pid").is_ok(),
        "nu didn't start the background process: {output:?}"
    );
    assert!(closed, "a background process outlived nu: {output:?}");
    assert_eq!(output.status.code(), Some(exit_code), "{output:?}");
    Ok(())
}

/// Run `code` as commands (`-c`) or as a script, optionally with `-i`.
fn code_args<'a>(
    dir: &Path,
    script: bool,
    interactive: bool,
    code: &'a str,
) -> Result<Vec<&'a str>> {
    let mut args = vec!["-n"];
    if interactive {
        args.push("-i");
    }
    if script {
        std::fs::write(dir.join("script.nu"), code)?;
        args.push("script.nu");
    } else {
        args.extend(["-c", code]);
    }
    Ok(args)
}

fn read(dir: &Path, file: &str) -> Result<String> {
    Ok(std::fs::read_to_string(dir.join(file))?)
}

/// Interrupting foreground work must not kill the REPL's background jobs.
#[test]
#[deps(NU)]
fn repl_interrupt_keeps_background_jobs() -> Result {
    Playground::setup("repl_interrupt_keeps_background_jobs", |dirs, _| {
        let dir = dirs.test().as_std_path();
        // With neither commands nor a script, Nu runs the REPL, which runs `-e` before it needs a
        // terminal. Catching the interrupt stands in for returning to the prompt.
        let code = format!(
            "
            job spawn {{ sleep 10min }} | ignore
            {CATCH_INTERRUPT}
            # Jobs that the interrupt kills would be gone by now.
            sleep 500ms
            job list | length | save jobs.txt
            "
        );
        let output = wait_for(nu(dir, &["-n", "--no-history", "-e", &code]).spawn()?)?;

        // Then the REPL stops, as there is no terminal.
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("STDIN is not a TTY"), "{output:?}");
        assert!(
            read(dir, "interrupt.txt")?.contains("interrupted"),
            "{output:?}"
        );
        assert_eq!(read(dir, "jobs.txt")?, "1", "{output:?}");
        Ok(())
    })
}

/// Commands and scripts don't return to a prompt, so an interrupt kills their background jobs,
/// even with `-i`, and even if they catch it.
#[rstest]
#[nu_test_support::test]
#[deps(NU)]
fn interrupt_outside_repl_kills_background_jobs(
    #[values(false, true)] script: bool,
    #[values(false, true)] interactive: bool,
) -> Result {
    Playground::setup("interrupt_outside_repl_kills_background_jobs", |dirs, _| {
        let dir = dirs.test().as_std_path();
        let code = format!(
            "
            job spawn {{ sleep 10min }} | ignore
            {CATCH_INTERRUPT}
            # The interrupt handler kills the jobs from its own thread.
            let deadline = (date now) + {}sec
            while (job list | is-not-empty) and (date now) < $deadline {{ sleep 10ms }}
            job list | length | save jobs.txt
            ",
            TIMEOUT.as_secs()
        );
        let args = code_args(dir, script, interactive, &code)?;
        let output = wait_for(nu(dir, &args).spawn()?)?;

        assert!(
            read(dir, "interrupt.txt")?.contains("interrupted"),
            "{output:?}"
        );
        assert_eq!(read(dir, "jobs.txt")?, "0", "{output:?}");
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        Ok(())
    })
}

/// When commands and scripts finish, fail or are interrupted, even with `-i`, their background
/// processes end with them.
#[rstest]
#[case::finish("", 0)]
#[case::error("error make {msg: boom}", 1)]
#[case::interrupt("kill --signal 2 $nu.pid; sleep 1min", 1)]
#[nu_test_support::test]
#[deps(NU)]
fn background_processes_end_with_commands_and_scripts(
    #[case] ending: &str,
    #[case] exit_code: i32,
    #[values(false, true)] script: bool,
    #[values(false, true)] interactive: bool,
) -> Result {
    Playground::setup(
        "background_processes_end_with_commands_and_scripts",
        |dirs, _| {
            let dir = dirs.test().as_std_path();
            let code = format!("{SPAWN_BACKGROUND_PROCESS}\n{ending}");
            let args = code_args(dir, script, interactive, &code)?;
            assert_nothing_outlives_nu(dir, &args, exit_code)
        },
    )
}

/// Nu exits without returning to a prompt when the startup files, the code or the REPL fail
/// before running; background processes started by then end with it.
#[rstest]
#[case::command_parse_error(&["--env-config", "spawn.nu", "-c", "("])]
#[case::script_parse_error(&["--env-config", "spawn.nu", "parse_error.nu"])]
#[case::config_error(&["--env-config", "spawn.nu", "--config", "error.nu", "-c", "print ok"])]
#[case::missing_config(&["--env-config", "spawn.nu", "--config", "missing.nu", "-c", "print ok"])]
#[case::repl_without_terminal(&["-n", "--no-history", "-e", SPAWN_BACKGROUND_PROCESS])]
#[nu_test_support::test]
#[deps(NU)]
fn background_processes_end_with_failed_startup(#[case] args: &[&str]) -> Result {
    Playground::setup(
        "background_processes_end_with_failed_startup",
        |dirs, sandbox| {
            sandbox.with_files(&[
                FileWithContent("spawn.nu", SPAWN_BACKGROUND_PROCESS),
                FileWithContent("parse_error.nu", "("),
                FileWithContent("error.nu", "error make {msg: boom}"),
            ]);
            assert_nothing_outlives_nu(dirs.test().as_std_path(), args, 1)
        },
    )
}
