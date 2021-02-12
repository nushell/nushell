use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::process::{Command, Stdio};

pub struct Kill;

#[derive(Deserialize)]
pub struct KillArgs {
    pub pid: Tagged<u64>,
    pub rest: Vec<Tagged<u64>>,
    pub force: Tagged<bool>,
    pub quiet: Tagged<bool>,
}

#[async_trait]
impl WholeStreamCommand for Kill {
    fn name(&self) -> &str {
        "kill"
    }

    fn signature(&self) -> Signature {
        Signature::build("kill")
            .required(
                "pid",
                SyntaxShape::Int,
                "process id of process that is to be killed",
            )
            .rest(SyntaxShape::Int, "rest of processes to kill")
            .switch("force", "forcefully kill the process", Some('f'))
            .switch("quiet", "won't print anything to the console", Some('q'))
    }

    fn usage(&self) -> &str {
        "Kill a process using the process id."
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        kill(args).await
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Kill the pid using the most memory",
                example: "ps | sort-by mem | last | kill $it.pid",
                result: None,
            },
            Example {
                description: "Force kill a given pid",
                example: "kill --force 12345",
                result: None,
            },
        ]
    }
}

async fn kill(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let (
        KillArgs {
            pid,
            rest,
            force,
            quiet,
        },
        ..,
    ) = args.process().await?;
    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("taskkill");

        if *force {
            cmd.arg("/F");
        }

        cmd.arg("/PID");
        cmd.arg(pid.item().to_string());

        // each pid must written as `/PID 0` otherwise
        // taskkill will act as `killall` unix command
        for id in &rest {
            cmd.arg("/PID");
            cmd.arg(id.item().to_string());
        }

        cmd
    } else {
        let mut cmd = Command::new("kill");

        if *force {
            cmd.arg("-9");
        }

        cmd.arg(pid.item().to_string());

        cmd.args(rest.iter().map(move |id| id.item().to_string()));

        cmd
    };

    // pipe everything to null
    if *quiet {
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }

    cmd.status().expect("failed to execute shell command");

    Ok(OutputStream::empty())
}

#[cfg(test)]
mod tests {
    use super::Kill;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Kill {})
    }
}
