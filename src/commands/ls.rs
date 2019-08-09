use crate::errors::ShellError;
use crate::prelude::*;

pub fn ls(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let shell_manager = args.shell_manager.clone();
    let args = args.evaluate_once(registry)?;
    shell_manager.ls(args)
}
