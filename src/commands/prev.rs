use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature};

use crate::commands::WholeStreamCommand;

pub struct Previous;

impl WholeStreamCommand for Previous {
    fn name(&self) -> &str {
        "p"
    }

    fn signature(&self) -> Signature {
        Signature::build("p")
    }

    fn usage(&self) -> &str {
        "Go to previous shell."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        previous(args, registry)
    }
}

fn previous(_args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::PreviousShell))].into())
}
