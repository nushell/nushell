use crate::errors::ShellError;
use crate::prelude::*;
use crate::shell::shell::Shell;

pub fn ls(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();

    env.ls(args.call_info, args.input)
}
