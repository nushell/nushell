use crate::commands::command::CommandAction;
use crate::errors::ShellError;
use crate::prelude::*;

pub fn prev(_args: CommandArgs, _registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    Ok(vec![Ok(ReturnSuccess::Action(CommandAction::PreviousShell))].into())
}
