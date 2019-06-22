use crate::errors::ShellError;
use crate::object::Value;
use crate::parser::{
    registry::{self, Args},
    Span, Spanned,
};
use crate::prelude::*;
use getset::Getters;
use std::path::PathBuf;

#[derive(Getters)]
#[get = "crate"]
pub struct CommandArgs {
    pub host: Arc<Mutex<dyn Host + Send>>,
    pub env: Arc<Mutex<VecDeque<Environment>>>,
    pub name_span: Option<Span>,
    pub args: Args,
    pub input: InputStream,
}

impl CommandArgs {
    pub fn nth(&self, pos: usize) -> Option<&Spanned<Value>> {
        self.args.nth(pos)
    }

    pub fn positional_iter(&self) -> impl Iterator<Item = &Spanned<Value>> {
        self.args.positional_iter()
    }

    pub fn expect_nth(&self, pos: usize) -> Result<&Spanned<Value>, ShellError> {
        self.args.expect_nth(pos)
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn get(&self, name: &str) -> Option<&Spanned<Value>> {
        self.args.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.args.has(name)
    }
}

pub struct SinkCommandArgs {
    pub ctx: Context,
    pub name_span: Option<Span>,
    pub args: Args,
    pub input: Vec<Value>,
}

#[derive(Debug)]
pub enum CommandAction {
    ChangePath(PathBuf),
    Enter(Value),
    Exit,
}

#[derive(Debug)]
pub enum ReturnValue {
    Value(Value),
    Action(CommandAction),
}

impl ReturnValue {
    crate fn change_cwd(path: PathBuf) -> ReturnValue {
        ReturnValue::Action(CommandAction::ChangePath(path))
    }
}

pub trait Command {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError>;
    fn name(&self) -> &str;

    fn config(&self) -> registry::CommandConfig {
        registry::CommandConfig {
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

    fn config(&self) -> registry::CommandConfig {
        registry::CommandConfig {
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
