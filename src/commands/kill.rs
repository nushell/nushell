use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::process::Command;

pub struct Kill;

#[derive(Deserialize)]
pub struct KillArgs {
    pub pid: Tagged<i64>,
    pub force: Tagged<bool>,
}

impl WholeStreamCommand for Kill {
    fn name(&self) -> &str {
        "kill"
    }

    fn signature(&self) -> Signature {
        Signature::build("kill")
            .required("pid", SyntaxShape::Int, "id of the process to be killed")
            .switch("force", "forcefully kill the process")
    }

    fn usage(&self) -> &str {
        "Kill a process using the process id."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        args.process(registry, kill)?.run()
    }
}

fn kill(
    KillArgs { pid, force }: KillArgs,
    _context: RunnableContext,
) -> Result<OutputStream, ShellError> {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");

        cmd.arg("/C");
        cmd.arg("taskkill");

        if *force {
            cmd.arg("/F");
        }

        // TODO try to avoid repeating this code

        // '/PID' must prefix every PID
        cmd.arg("/PID")
            .arg(pid.item().to_string())
            .status()
            .expect("failed to execute process");
    } else if cfg!(unix) {
        let mut cmd = Command::new("/bin/sh");

        cmd.arg("-c").arg("kill");

        if *force {
            cmd.arg("-9");
        }

        cmd.arg(pid.item().to_string())
            .status()
            .expect("failed to execute process");
    }

    Ok(OutputStream::empty())
}
