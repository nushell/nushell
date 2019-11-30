use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature};

pub struct Next;

impl WholeStreamCommand for Next {
    fn name(&self) -> &str {
        "n"
    }

    fn signature(&self) -> Signature {
        Signature::build("n")
    }

    fn usage(&self) -> &str {
        "Go to next shell."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        next(args, registry)
    }
}

fn next(_args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::NextShell))].into())
}
