use std::{io::Write, process::Command};

use rexpect::{
    error::Error,
    session::{spawn_command, PtyReplSession},
};

mod signals;

fn nu_binary() -> String {
    nu_test_support::fs::executable_path()
        .into_os_string()
        .to_string_lossy()
        .into_owned()
}

fn spawn_nu(timeout: Option<u64>) -> Result<PtyReplSession, Error> {
    let mut config_dir = nu_test_support::fs::root();
    config_dir.extend(["tests", "rexpect", "config"]);

    let mut command = Command::new(nu_binary());
    command
        .arg("--config")
        .arg(config_dir.join("config.nu"))
        .arg("--env-config")
        .arg(config_dir.join("env.nu"));

    Ok(PtyReplSession {
        prompt: ">> ".into(),
        pty_session: spawn_command(command, timeout)?,
        quit_command: None,
        echo_on: false,
    })
}

trait NuReplExt {
    fn send_nu_line(&mut self, line: &str) -> Result<usize, Error>;

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

    fn exit(&mut self) -> Result<(), Error> {
        self.send_nu_line("exit")?;
        Ok(())
    }
}

#[test]
fn echo_back() -> Result<(), Error> {
    let mut p = spawn_nu(Some(3000))?;
    p.wait_for_prompt()?;

    p.send_nu_line("'some text'")?;
    p.exp_string("some text")?;

    p.exit()
}
