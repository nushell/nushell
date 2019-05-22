use crate::errors::ShellError;
use crate::object::Value;
use crate::prelude::*;
use crate::Context;
use std::path::PathBuf;

pub struct CommandArgs<'caller> {
    pub host: &'caller mut dyn Host,
    pub env: &'caller crate::Environment,
    pub args: Vec<Value>,
    pub input: VecDeque<Value>,
}

impl CommandArgs<'caller> {
    crate fn from_context(
        ctx: &'caller mut Context,
        args: Vec<Value>,
        input: VecDeque<Value>,
    ) -> CommandArgs<'caller> {
        CommandArgs {
            host: &mut ctx.host,
            env: &ctx.env,
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
    crate fn single(value: Value) -> VecDeque<ReturnValue> {
        let mut v = VecDeque::new();
        v.push_back(ReturnValue::Value(value));
        v
    }

    crate fn change_cwd(path: PathBuf) -> ReturnValue {
        ReturnValue::Action(CommandAction::ChangeCwd(path))
    }
}

pub trait Command {
    fn run(&self, args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError>;
}

impl<F> Command for F
where
    F: Fn(CommandArgs<'_>) -> Result<VecDeque<ReturnValue>, ShellError>,
{
    fn run(&self, args: CommandArgs<'caller>) -> Result<VecDeque<ReturnValue>, ShellError> {
        self(args)
    }
}
