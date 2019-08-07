use crate::errors::ShellError;
use crate::prelude::*;

pub fn cd(args: CommandArgs) -> Result<OutputStream, ShellError> {
    args.shell_manager.cd(args.call_info, args.input)
}
