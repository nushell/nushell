use crate::prelude::*;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

#[cfg(unix)]
use nu_source::Tagged;
#[cfg(unix)]
use std::path::PathBuf;

pub struct Exec;

#[cfg(unix)]
pub struct ExecArgs {
    pub command: Tagged<PathBuf>,
    pub rest: Vec<Tagged<String>>,
}

impl WholeStreamCommand for Exec {
    fn name(&self) -> &str {
        "exec"
    }

    fn signature(&self) -> Signature {
        Signature::build("exec")
            .required("command", SyntaxShape::FilePath, "the command to execute")
            .rest(
                "rest",
                SyntaxShape::GlobPattern,
                "any additional arguments for the command",
            )
    }

    fn usage(&self) -> &str {
        "Execute a command, replacing the current process."
    }

    fn extra_usage(&self) -> &str {
        "Currently supported only on Unix-based systems."
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        exec(args)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Execute external 'ps aux' tool",
                example: "exec ps aux",
                result: None,
            },
            Example {
                description: "Execute 'nautilus'",
                example: "exec nautilus",
                result: None,
            },
        ]
    }
}

#[cfg(unix)]
fn exec(args: CommandArgs) -> Result<OutputStream, ShellError> {
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    let name = args.call_info.name_tag.clone();

    let args = ExecArgs {
        command: args.req(0)?,
        rest: args.rest(1)?,
    };

    let mut command = Command::new(args.command.item);
    for tagged_arg in args.rest {
        command.arg(tagged_arg.item);
    }

    let err = command.exec(); // this replaces our process, should not return

    Err(ShellError::labeled_error(
        "Error on exec",
        err.to_string(),
        &name,
    ))
}

#[cfg(not(unix))]
fn exec(args: CommandArgs) -> Result<OutputStream, ShellError> {
    Err(ShellError::labeled_error(
        "Error on exec",
        "exec is not supported on your platform",
        &args.call_info.name_tag,
    ))
}
