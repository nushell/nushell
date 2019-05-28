use crate::errors::ShellError;
use crate::object::Value;
use crate::parser::CommandConfig;
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
    fn name(&self) -> &str;

    fn config(&self) -> CommandConfig {
        CommandConfig {
            name: self.name().to_string(),
            mandatory_positional: vec![],
            optional_positional: vec![],
            rest_positional: true,
            named: indexmap::IndexMap::new(),
        }
    }
}

pub struct FnCommand {
    name: String,
    func: fn(CommandArgs) -> Result<OutputStream, ShellError>,
}

impl Command for FnCommand {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        (self.func)(args)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub fn command(
    name: &str,
    func: fn(CommandArgs) -> Result<OutputStream, ShellError>,
) -> Arc<dyn Command> {
    Arc::new(FnCommand {
        name: name.to_string(),
        func,
    })
}
