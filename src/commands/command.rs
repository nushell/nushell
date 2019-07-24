use crate::context::{SourceMap, SpanSource};
use crate::errors::ShellError;
use crate::evaluate::Scope;
use crate::object::Value;
use crate::parser::hir;
use crate::parser::{registry, Span, Spanned};
use crate::prelude::*;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: hir::Call,
    pub source: Text,
    pub source_map: SourceMap,
    pub name_span: Option<Span>,
}

impl ToDebug for UnevaluatedCallInfo {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        self.args.fmt_debug(f, source)
    }
}

impl UnevaluatedCallInfo {
    fn evaluate(
        self,
        registry: &registry::CommandRegistry,
        scope: &Scope,
    ) -> Result<CallInfo, ShellError> {
        let args = self.args.evaluate(registry, scope, &self.source)?;

        Ok(CallInfo {
            args,
            source_map: self.source_map,
            name_span: self.name_span,
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CallInfo {
    pub args: registry::EvaluatedArgs,
    pub source_map: SourceMap,
    pub name_span: Option<Span>,
}

#[derive(Getters)]
#[get = "crate"]
pub struct CommandArgs {
    pub host: Arc<Mutex<dyn Host>>,
    pub env: Arc<Mutex<Environment>>,
    pub call_info: UnevaluatedCallInfo,
    pub input: InputStream,
}

impl ToDebug for CommandArgs {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        self.call_info.fmt_debug(f, source)
    }
}

impl CommandArgs {
    pub fn evaluate_once(
        self,
        registry: &registry::CommandRegistry,
    ) -> Result<EvaluatedStaticCommandArgs, ShellError> {
        let host = self.host.clone();
        let env = self.env.clone();
        let input = self.input;
        let call_info = self.call_info.evaluate(registry, &Scope::empty())?;

        Ok(EvaluatedStaticCommandArgs::new(host, env, call_info, input))
    }

    pub fn name_span(&self) -> Option<Span> {
        self.call_info.name_span
    }
}

pub struct EvaluatedStaticCommandArgs {
    pub args: EvaluatedCommandArgs,
    pub input: InputStream,
}

impl Deref for EvaluatedStaticCommandArgs {
    type Target = EvaluatedCommandArgs;
    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

impl EvaluatedStaticCommandArgs {
    pub fn new(
        host: Arc<Mutex<dyn Host>>,
        env: Arc<Mutex<Environment>>,
        call_info: CallInfo,
        input: impl Into<InputStream>,
    ) -> EvaluatedStaticCommandArgs {
        EvaluatedStaticCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                env,
                call_info,
            },
            input: input.into(),
        }
    }

    pub fn name_span(&self) -> Option<Span> {
        self.args.call_info.name_span
    }

    pub fn parts(self) -> (InputStream, registry::EvaluatedArgs) {
        let EvaluatedStaticCommandArgs { args, input } = self;

        (input, args.call_info.args)
    }
}

#[derive(Getters)]
#[get = "pub"]
pub struct EvaluatedFilterCommandArgs {
    args: EvaluatedCommandArgs,
    #[allow(unused)]
    input: Spanned<Value>,
}

impl Deref for EvaluatedFilterCommandArgs {
    type Target = EvaluatedCommandArgs;
    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

impl EvaluatedFilterCommandArgs {
    pub fn new(
        host: Arc<Mutex<dyn Host>>,
        env: Arc<Mutex<Environment>>,
        call_info: CallInfo,
        input: Spanned<Value>,
    ) -> EvaluatedFilterCommandArgs {
        EvaluatedFilterCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                env,
                call_info,
            },
            input,
        }
    }
}

#[derive(Getters)]
#[get = "crate"]
pub struct EvaluatedCommandArgs {
    pub host: Arc<Mutex<dyn Host>>,
    pub env: Arc<Mutex<Environment>>,
    pub call_info: CallInfo,
}

impl EvaluatedCommandArgs {
    pub fn call_args(&self) -> &registry::EvaluatedArgs {
        &self.call_info.args
    }

    pub fn nth(&self, pos: usize) -> Option<&Spanned<Value>> {
        self.call_info.args.nth(pos)
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

impl SinkCommandArgs {
    pub fn name_span(&self) -> Option<Span> {
        self.call_info.name_span
    }
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

pub trait Command: Send + Sync {
    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError>;
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

#[allow(unused)]
pub struct FnFilterCommand {
    name: String,
    func: fn(EvaluatedFilterCommandArgs) -> Result<OutputStream, ShellError>,
}

impl Command for FnFilterCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let CommandArgs {
            host,
            env,
            call_info,
            input,
        } = args;

        let host: Arc<Mutex<dyn Host>> = host.clone();
        let env: Arc<Mutex<Environment>> = env.clone();
        let registry: registry::CommandRegistry = registry.clone();
        let func = self.func;

        let result = input.values.map(move |it| {
            let registry = registry.clone();
            let call_info = match call_info
                .clone()
                .evaluate(&registry, &Scope::it_value(it.clone()))
            {
                Err(err) => return OutputStream::from(vec![Err(err)]).values,
                Ok(args) => args,
            };

            let args = EvaluatedFilterCommandArgs::new(host.clone(), env.clone(), call_info, it);

            match func(args) {
                Err(err) => return OutputStream::from(vec![Err(err)]).values,
                Ok(stream) => stream.values,
            }
        });

        let result = result.flatten();
        let result: BoxStream<ReturnValue> = result.boxed();

        Ok(result.into())
    }
}

pub struct FnRawCommand {
    name: String,
    func: Box<
        dyn Fn(CommandArgs, &registry::CommandRegistry) -> Result<OutputStream, ShellError>
            + Send
            + Sync,
    >,
}

impl Command for FnRawCommand {
    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        (self.func)(args, registry)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub fn command(
    name: &str,
    func: Box<
        dyn Fn(CommandArgs, &registry::CommandRegistry) -> Result<OutputStream, ShellError>
            + Send
            + Sync,
    >,
) -> Arc<dyn Command> {
    Arc::new(FnRawCommand {
        name: name.to_string(),
        func,
    })
}

#[allow(unused)]
pub fn filter(
    name: &str,
    func: fn(EvaluatedFilterCommandArgs) -> Result<OutputStream, ShellError>,
) -> Arc<dyn Command> {
    Arc::new(FnFilterCommand {
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
