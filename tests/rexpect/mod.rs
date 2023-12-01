use std::{io::Write, process::Command};

use nu_test_support::fs::executable_path;
use rexpect::{
    error::Error,
    process::PtyProcess,
    session::{spawn_command, PtyReplSession},
};

mod signals;

fn nu_binary() -> String {
    // commands will fail later anyways if `to_string_lossy` performs lossy conversion,
    // so we just panic here on `unwrap` instead
    executable_path().into_os_string().into_string().unwrap()
}

/// Spawn an interactive nu repl session
fn spawn_nu_repl(timeout: Option<u64>) -> Result<PtyReplSession, Error> {
    let mut config_dir = nu_test_support::fs::root();
    config_dir.extend(["tests", "rexpect", "config"]);

    let mut command = Command::new(executable_path());
    command
        .arg("--config")
        .arg(config_dir.join("config.nu"))
        .arg("--env-config")
        .arg(config_dir.join("env.nu"));

    let mut session = PtyReplSession {
        prompt: "<REXPECT_PROMPT>".into(),
        pty_session: spawn_command(command, timeout)?,
        quit_command: None,
        echo_on: false,
    };

    session.handle_prompt()?;

    Ok(session)
}

/// Spawn a non-interactive nu process with `command` supplied to the `-c` flag
fn spawn_nu(command: &str) -> Result<PtyProcess, Error> {
    let mut cmd = Command::new(executable_path());
    cmd.arg("-n").arg("-c").arg(command);
    PtyProcess::new(cmd)
}

trait NuReplExt {
    /// Send a line to the nu repl (and flush output), returning the number of bytes written
    fn send_nu_line(&mut self, line: &str) -> Result<usize, Error>;

    /// Wait for the prompt to appear, handling any necessary communication with reedline
    fn handle_prompt(&mut self) -> Result<(), Error>;

    /// Exit the nu repl early to avoid waiting for the timeout to occur
    fn exit(&mut self) -> Result<(), Error>;
}

impl NuReplExt for PtyReplSession {
    fn send_nu_line(&mut self, line: &str) -> Result<usize, Error> {
        let len = self.send(line)?;
        let len = len + self.writer.write(&[b'\r'])?;
        self.flush()?;
        if self.echo_on {
            self.exp_string(line)?;
        }
        Ok(len)
    }

    fn handle_prompt(&mut self) -> Result<(), Error> {
        // reedline queries the cursor position before drawing the prompt
        self.exp_string("\x1B[6n")?;

        // always reply with (1, 1)?
        self.send("\x1B[1;1R")?;
        self.flush()?;

        // prompt will be drawn after responding to the query
        self.wait_for_prompt()?;

        Ok(())
    }

    fn exit(&mut self) -> Result<(), Error> {
        self.send_nu_line("exit")?;
        Ok(())
    }
}
