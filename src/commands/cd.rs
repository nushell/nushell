use crate::errors::ShellError;
use crate::prelude::*;
use crate::shell::shell::Shell;

pub fn cd(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let env = args.env.lock().unwrap();

    env.cd(args.call_info, args.input)
}
