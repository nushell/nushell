use crate::errors::ShellError;
use crate::prelude::*;

pub fn ls(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();

    env.last().unwrap().ls(args.call_info, args.input)
}
