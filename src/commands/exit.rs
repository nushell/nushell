use crate::commands::command::WholeStreamCommand;
use crate::context::CommandRegistry;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{CommandAction, ReturnSuccess, Signature};

pub struct Exit;

impl WholeStreamCommand for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit").switch("now", "exit out of the shell immediately")
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
