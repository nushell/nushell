use crate::commands::command::CommandAction;
use crate::errors::ShellError;
use crate::prelude::*;

use crate::commands::WholeStreamCommand;

pub struct Previous;

impl WholeStreamCommand for Previous {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        previous(args, registry)
    }

    fn name(&self) -> &str {
        "p"
    }

    fn signature(&self) -> Signature {
        Signature::build("p")
    }
}

fn previous(_args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::PreviousShell))].into())
}
