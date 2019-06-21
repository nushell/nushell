use crate::commands::command::CommandAction;
use crate::errors::ShellError;
use crate::prelude::*;

pub fn exit(_args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut stream = VecDeque::new();
    stream.push_back(ReturnValue::Action(CommandAction::Exit));
    Ok(stream.boxed())
}
