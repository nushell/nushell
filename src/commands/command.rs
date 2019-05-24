use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use std::path::PathBuf;

pub struct CommandArgs {
    pub host: Arc<Mutex<dyn Host + Send>>,
    pub env: Arc<Mutex<Environment>>,
    pub args: Vec<Value>,
    pub input: InputStream,
}

impl CommandArgs {
    crate fn from_context(
        ctx: &'caller mut Context,
        args: Vec<Value>,
        input: InputStream,
    ) -> CommandArgs {
        CommandArgs {
            host: ctx.host.clone(),
            env: ctx.env.clone(),
            args,
            input,
        }
    }
}

#[derive(Debug)]
pub enum CommandAction {
    ChangeCwd(PathBuf),
}

#[derive(Debug)]
pub enum ReturnValue {
    Value(Value),
    Action(CommandAction),
}

impl ReturnValue {
    crate fn change_cwd(path: PathBuf) -> ReturnValue {
        ReturnValue::Action(CommandAction::ChangeCwd(path))
    }
}

pub trait Command {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError>;
}

impl<F> Command for F
where
    F: Fn(CommandArgs) -> Result<OutputStream, ShellError>,
{
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        self(args)
    }
}
