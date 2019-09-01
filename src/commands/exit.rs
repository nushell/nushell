use crate::commands::command::{CommandAction, WholeStreamCommand};
use crate::errors::ShellError;
use crate::parser::registry::{CommandRegistry, Signature};
use crate::prelude::*;

pub struct Exit;

impl WholeStreamCommand for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit")
            .switch("now")
    }

    fn usage(&self) -> &str {
        "Exit the current shell (or all shells)"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        exit(args, registry)
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
