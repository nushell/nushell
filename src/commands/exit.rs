use crate::commands::command::CommandAction;
use crate::errors::ShellError;
use crate::object::{Primitive, Value};
use crate::parser::lexer::Spanned;
use crate::prelude::*;
use std::path::{Path, PathBuf};

pub fn exit(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let mut stream = VecDeque::new();
    stream.push_back(ReturnValue::Action(CommandAction::Exit));
    Ok(stream.boxed())
}
