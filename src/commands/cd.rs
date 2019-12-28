use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use nu_errors::ShellError;
use nu_macros::signature;
use nu_protocol::{Signature, SyntaxShape};

pub struct Cd;

impl WholeStreamCommand for Cd {
    fn name(&self) -> &str {
        "cd"
    }

    fn signature(&self) -> Signature {
        signature! {
            def cd {
                "the directory to change to"
                directory(optional Path) - "the directory to change to"
            }
        }
        // Signature::build("cd").optional(
        //     "directory",
        //     SyntaxShape::Path,
        //     "the directory to change to",
        // )
    }

    fn usage(&self) -> &str {
        "Change to a new path."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        cd(args, registry)
    }
}

fn cd(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let shell_manager = args.shell_manager.clone();
    let args = args.evaluate_once(registry)?;
    shell_manager.cd(args)
}
