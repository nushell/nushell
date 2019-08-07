use crate::errors::ShellError;
use crate::prelude::*;

pub fn ls(args: CommandArgs) -> Result<OutputStream, ShellError> {
    args.shell_manager.ls(args.call_info, args.input)
}
