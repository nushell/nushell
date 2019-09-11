use crate::commands::WholeStreamCommand;
use crate::errors::ShellError;
use crate::prelude::*;

pub struct CD;

impl WholeStreamCommand for CD {
    fn name(&self) -> &str {
        "cd"
    }

    fn signature(&self) -> Signature {
        Signature::build("cd")
            .optional("directory", SyntaxType::Path)
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
