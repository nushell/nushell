use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tagged;
use std::process::{Command, Stdio};

pub struct Kill;

impl WholeStreamCommand for Kill {
    fn name(&self) -> &str {
        "kill"
    }

    fn signature(&self) -> Signature {
        let signature = Signature::build("kill")
            .required(
                "pid",
                SyntaxShape::Int,
                "process id of process that is to be killed",
            )
            .rest("rest", SyntaxShape::Int, "rest of processes to kill")
            .switch("force", "forcefully kill the process", Some('f'))
            .switch("quiet", "won't print anything to the console", Some('q'));

        if cfg!(windows) {
            return signature;
        }

        signature.named(
            "signal",
            SyntaxShape::Int,
            "signal decimal number to be sent instead of the default 15 (unsupported on Windows)",
            Some('s'),
        )
    }

    fn usage(&self) -> &str {
        "Kill a process using the process id."
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        kill(args)
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
            Example {
                description: "Send INT signal",
                example: "kill -s 2 12345",
                result: None,
            },
        ]
    }
}

fn kill(args: CommandArgs) -> Result<ActionStream, ShellError> {
    let pid: Tagged<u64> = args.req(0)?;
    let rest: Vec<Tagged<u64>> = args.rest(1)?;
    let force: Option<Tagged<bool>> = args.get_flag("force")?;
    let quiet: bool = args.has_flag("quiet");
    let signal: Option<Tagged<u32>> = args.get_flag("signal")?;

    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("taskkill");

        if matches!(force, Some(Tagged { item: true, .. })) {
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

        if matches!(force, Some(Tagged { item: true, .. })) {
            if let Some(signal_value) = signal {
                return Err(ShellError::labeled_error_with_secondary(
                    "mixing force and signal options is not supported",
                    "signal option",
                    signal_value.tag(),
                    "force option",
                    force.expect("internal error: expected value").tag(),
                ));
            }
            cmd.arg("-9");
        } else if let Some(signal_value) = signal {
            cmd.arg(format!("-{}", signal_value.item().to_string()));
        }

        cmd.arg(pid.item().to_string());

        cmd.args(rest.iter().map(move |id| id.item().to_string()));

        cmd
    };

    // pipe everything to null
    if quiet {
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
    }

    cmd.status().expect("failed to execute shell command");

    Ok(ActionStream::empty())
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
