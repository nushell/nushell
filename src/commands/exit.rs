use crate::commands::command::{CommandAction, StaticCommand};
use crate::errors::ShellError;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;

pub struct Exit;

impl StaticCommand for Exit {
    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        exit(args, registry)
    }

    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit").switch("now")
    }
}

pub fn exit(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    if args.call_info.args.has("now") {
        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::Exit))].into())
    } else {
        Ok(vec![Ok(ReturnSuccess::Action(CommandAction::LeaveShell))].into())
    }
}
