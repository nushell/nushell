use crate::errors::ShellError;
use crate::object::Value;
use crate::parser::CommandConfig;
use crate::prelude::*;
use core::future::Future;
use std::path::PathBuf;

pub struct CommandArgs {
    pub host: Arc<Mutex<dyn Host + Send>>,
    pub env: Arc<Mutex<Environment>>,
    pub positional: Vec<Value>,
    pub named: indexmap::IndexMap<String, Value>,
    pub input: InputStream,
}

impl CommandArgs {
    crate fn from_context(
        ctx: &'caller mut Context,
        positional: Vec<Value>,
        input: InputStream,
    ) -> CommandArgs {
        CommandArgs {
            host: ctx.host.clone(),
            env: ctx.env.clone(),
            positional,
            named: indexmap::IndexMap::default(),
            input,
        }
    }
}

pub struct SinkCommandArgs {
    pub ctx: Context,
    pub positional: Vec<Value>,
    pub named: indexmap::IndexMap<String, Value>,
    pub input: Vec<Value>,
}

impl SinkCommandArgs {
    crate fn from_context(
        ctx: &'caller mut Context,
        positional: Vec<Value>,
        input: Vec<Value>,
    ) -> SinkCommandArgs {
        SinkCommandArgs {
            ctx: ctx.clone(),
            positional,
            named: indexmap::IndexMap::default(),
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

pub trait Sink {
    fn run(&self, args: SinkCommandArgs) -> Result<(), ShellError>;
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

pub struct FnSink {
    name: String,
    func: fn(SinkCommandArgs) -> Result<(), ShellError>,
}

impl Sink for FnSink {
    fn run(&self, args: SinkCommandArgs) -> Result<(), ShellError> {
        (self.func)(args)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub fn sink(name: &str, func: fn(SinkCommandArgs) -> Result<(), ShellError>) -> Arc<dyn Sink> {
    Arc::new(FnSink {
        name: name.to_string(),
        func,
    })
}
