use crate::context::SourceMap;
use crate::context::SpanSource;
use crate::errors::ShellError;
use crate::object::Value;
use crate::parser::{
    registry::{self, Args},
    Span, Spanned,
};
use crate::prelude::*;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
pub struct CallInfo {
    pub args: Args,
    pub source_map: SourceMap,
    pub name_span: Option<Span>,
}

#[derive(Getters)]
#[get = "crate"]
pub struct CommandArgs {
    pub host: Arc<Mutex<dyn Host + Send>>,
    pub env: Arc<Mutex<Environment>>,
    pub call_info: CallInfo,
    pub input: InputStream,
}

impl CommandArgs {
    pub fn nth(&self, pos: usize) -> Option<&Spanned<Value>> {
        self.call_info.args.nth(pos)
    }

    pub fn positional_iter(&self) -> impl Iterator<Item = &Spanned<Value>> {
        self.call_info.args.positional_iter()
    }

    pub fn expect_nth(&self, pos: usize) -> Result<&Spanned<Value>, ShellError> {
        self.call_info.args.expect_nth(pos)
    }

    pub fn len(&self) -> usize {
        self.call_info.args.len()
    }

    pub fn get(&self, name: &str) -> Option<&Spanned<Value>> {
        self.call_info.args.get(name)
    }

    #[allow(unused)]
    pub fn has(&self, name: &str) -> bool {
        self.call_info.args.has(name)
    }
}

pub struct SinkCommandArgs {
    pub ctx: Context,
    pub call_info: CallInfo,
    pub input: Vec<Spanned<Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandAction {
    ChangePath(PathBuf),
    AddSpanSource(Uuid, SpanSource),
    Exit,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ReturnSuccess {
    Value(Spanned<Value>),
    Action(CommandAction),
}

pub type ReturnValue = Result<ReturnSuccess, ShellError>;

impl From<Spanned<Value>> for ReturnValue {
    fn from(input: Spanned<Value>) -> ReturnValue {
        Ok(ReturnSuccess::Value(input))
    }
}

impl ReturnSuccess {
    pub fn change_cwd(path: PathBuf) -> ReturnValue {
        Ok(ReturnSuccess::Action(CommandAction::ChangePath(path)))
    }

    pub fn value(input: impl Into<Spanned<Value>>) -> ReturnValue {
        Ok(ReturnSuccess::Value(input.into()))
    }

    pub fn action(input: CommandAction) -> ReturnValue {
        Ok(ReturnSuccess::Action(input))
    }

    pub fn spanned_value(input: Value, span: Span) -> ReturnValue {
        Ok(ReturnSuccess::Value(Spanned::from_item(input, span)))
    }
}

pub trait Command {
    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError>;
    fn name(&self) -> &str;

    fn config(&self) -> registry::CommandConfig {
        registry::CommandConfig {
            name: self.name().to_string(),
            positional: vec![],
            rest_positional: true,
            named: indexmap::IndexMap::new(),
            is_filter: true,
            is_sink: false,
        }
    }
}

pub trait Sink {
    fn run(&self, args: SinkCommandArgs) -> Result<(), ShellError>;
    fn name(&self) -> &str;

    fn config(&self) -> registry::CommandConfig {
        registry::CommandConfig {
            name: self.name().to_string(),
            positional: vec![],
            rest_positional: true,
            named: indexmap::IndexMap::new(),
            is_filter: false,
            is_sink: true,
        }
    }
}

pub struct FnCommand {
    name: String,
    func: Box<dyn Fn(CommandArgs) -> Result<OutputStream, ShellError>>,
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
    func: Box<dyn Fn(CommandArgs) -> Result<OutputStream, ShellError>>,
) -> Arc<dyn Command> {
    Arc::new(FnCommand {
        name: name.to_string(),
        func,
    })
}

pub struct FnSink {
    name: String,
    func: Box<dyn Fn(SinkCommandArgs) -> Result<(), ShellError>>,
}

impl Sink for FnSink {
    fn run(&self, args: SinkCommandArgs) -> Result<(), ShellError> {
        (self.func)(args)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub fn sink(
    name: &str,
    func: Box<dyn Fn(SinkCommandArgs) -> Result<(), ShellError>>,
) -> Arc<dyn Sink> {
    Arc::new(FnSink {
        name: name.to_string(),
        func,
    })
}
