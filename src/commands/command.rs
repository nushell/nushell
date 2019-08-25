use crate::context::{SourceMap, SpanSource};
use crate::errors::ShellError;
use crate::evaluate::Scope;
use crate::object::Value;
use crate::parser::hir;
use crate::parser::{registry, ConfigDeserializer};
use crate::prelude::*;
use derive_new::new;
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
    pub name_span: Span,
}

impl ToDebug for UnevaluatedCallInfo {
    fn fmt_debug(&self, f: &mut fmt::Formatter, source: &str) -> fmt::Result {
        self.args.fmt_debug(f, source)
    }
}

impl UnevaluatedCallInfo {
    pub fn evaluate(
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

    pub fn has_it_or_block(&self) -> bool {
        use hir::RawExpression;
        use hir::Variable;

        if let Some(positional) = &self.args.positional() {
            for pos in positional {
                match pos {
                    Tagged {
                        item: RawExpression::Variable(Variable::It(_)),
                        ..
                    } => {
                        return true;
                    }
                    Tagged {
                        item: RawExpression::Block(_),
                        ..
                    } => {
                        return true;
                    }
                    _ => {}
                }
            }
        }

        false
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CallInfo {
    pub args: registry::EvaluatedArgs,
    pub source_map: SourceMap,
    pub name_span: Span,
}

impl CallInfo {
    pub fn process<'de, T: Deserialize<'de>>(
        &self,
        shell_manager: &ShellManager,
        callback: fn(T, &RunnablePerItemContext) -> Result<OutputStream, ShellError>,
    ) -> Result<RunnablePerItemArgs<T>, ShellError> {
        let mut deserializer = ConfigDeserializer::from_call_info(self.clone());

        Ok(RunnablePerItemArgs {
            args: T::deserialize(&mut deserializer)?,
            context: RunnablePerItemContext {
                shell_manager: shell_manager.clone(),
                name: self.name_span,
            },
            callback,
        })
    }
}

#[derive(Getters)]
#[get = "crate"]
pub struct CommandArgs {
    pub host: Arc<Mutex<dyn Host>>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
    pub input: InputStream,
}

#[derive(Getters, Clone)]
#[get = "crate"]
pub struct RawCommandArgs {
    pub host: Arc<Mutex<dyn Host>>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
}

impl RawCommandArgs {
    pub fn with_input(self, input: Vec<Tagged<Value>>) -> CommandArgs {
        CommandArgs {
            host: self.host,
            shell_manager: self.shell_manager,
            call_info: self.call_info,
            input: input.into(),
        }
    }
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
    ) -> Result<EvaluatedWholeStreamCommandArgs, ShellError> {
        let host = self.host.clone();
        let shell_manager = self.shell_manager.clone();
        let input = self.input;
        let call_info = self.call_info.evaluate(registry, &Scope::empty())?;

        Ok(EvaluatedWholeStreamCommandArgs::new(
            host,
            shell_manager,
            call_info,
            input,
        ))
    }

    // pub fn name_span(&self) -> Span {
    //     self.call_info.name_span
    // }

    pub fn process<'de, T: Deserialize<'de>>(
        self,
        registry: &CommandRegistry,
        callback: fn(T, RunnableContext) -> Result<OutputStream, ShellError>,
    ) -> Result<RunnableArgs<T>, ShellError> {
        let shell_manager = self.shell_manager.clone();
        let source_map = self.call_info.source_map.clone();
        let host = self.host.clone();
        let args = self.evaluate_once(registry)?;
        let (input, args) = args.split();
        let name_span = args.call_info.name_span;
        let mut deserializer = ConfigDeserializer::from_call_info(args.call_info);

        Ok(RunnableArgs {
            args: T::deserialize(&mut deserializer)?,
            context: RunnableContext {
                input: input,
                commands: registry.clone(),
                shell_manager,
                name: name_span,
                source_map,
                host,
            },
            callback,
        })
    }

    pub fn process_raw<'de, T: Deserialize<'de>>(
        self,
        registry: &CommandRegistry,
        callback: fn(T, RunnableContext, RawCommandArgs) -> Result<OutputStream, ShellError>,
    ) -> Result<RunnableRawArgs<T>, ShellError> {
        let raw_args = RawCommandArgs {
            host: self.host.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: self.call_info.clone(),
        };

        let shell_manager = self.shell_manager.clone();
        let source_map = self.call_info.source_map.clone();
        let host = self.host.clone();
        let args = self.evaluate_once(registry)?;
        let (input, args) = args.split();
        let name_span = args.call_info.name_span;
        let mut deserializer = ConfigDeserializer::from_call_info(args.call_info);

        Ok(RunnableRawArgs {
            args: T::deserialize(&mut deserializer)?,
            context: RunnableContext {
                input: input,
                commands: registry.clone(),
                shell_manager,
                name: name_span,
                source_map,
                host,
            },
            raw_args,
            callback,
        })
    }
}

pub struct RunnablePerItemContext {
    pub shell_manager: ShellManager,
    pub name: Span,
}

impl RunnablePerItemContext {
    pub fn cwd(&self) -> PathBuf {
        PathBuf::from(self.shell_manager.path())
    }
}

pub struct RunnableContext {
    pub input: InputStream,
    pub shell_manager: ShellManager,
    pub host: Arc<Mutex<dyn Host>>,
    pub commands: CommandRegistry,
    pub source_map: SourceMap,
    pub name: Span,
}

impl RunnableContext {
    #[allow(unused)]
    pub fn cwd(&self) -> PathBuf {
        PathBuf::from(self.shell_manager.path())
    }

    pub fn expect_command(&self, name: &str) -> Arc<Command> {
        self.commands
            .get_command(name)
            .expect(&format!("Expected command {}", name))
    }
}

pub struct RunnablePerItemArgs<T> {
    args: T,
    context: RunnablePerItemContext,
    callback: fn(T, &RunnablePerItemContext) -> Result<OutputStream, ShellError>,
}

impl<T> RunnablePerItemArgs<T> {
    pub fn run(self) -> Result<OutputStream, ShellError> {
        (self.callback)(self.args, &self.context)
    }
}

pub struct RunnableArgs<T> {
    args: T,
    context: RunnableContext,
    callback: fn(T, RunnableContext) -> Result<OutputStream, ShellError>,
}

impl<T> RunnableArgs<T> {
    pub fn run(self) -> Result<OutputStream, ShellError> {
        (self.callback)(self.args, self.context)
    }
}

pub struct RunnableRawArgs<T> {
    args: T,
    raw_args: RawCommandArgs,
    context: RunnableContext,
    callback: fn(T, RunnableContext, RawCommandArgs) -> Result<OutputStream, ShellError>,
}

impl<T> RunnableRawArgs<T> {
    pub fn run(self) -> OutputStream {
        match (self.callback)(self.args, self.context, self.raw_args) {
            Ok(stream) => stream,
            Err(err) => OutputStream::one(Err(err)),
        }
    }
}

pub struct EvaluatedWholeStreamCommandArgs {
    pub args: EvaluatedCommandArgs,
    pub input: InputStream,
}

impl Deref for EvaluatedWholeStreamCommandArgs {
    type Target = EvaluatedCommandArgs;
    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

impl EvaluatedWholeStreamCommandArgs {
    pub fn new(
        host: Arc<Mutex<dyn Host>>,
        shell_manager: ShellManager,
        call_info: CallInfo,
        input: impl Into<InputStream>,
    ) -> EvaluatedWholeStreamCommandArgs {
        EvaluatedWholeStreamCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                shell_manager,
                call_info,
            },
            input: input.into(),
        }
    }

    pub fn name_span(&self) -> Span {
        self.args.call_info.name_span
    }

    pub fn parts(self) -> (InputStream, registry::EvaluatedArgs) {
        let EvaluatedWholeStreamCommandArgs { args, input } = self;

        (input, args.call_info.args)
    }

    pub fn split(self) -> (InputStream, EvaluatedCommandArgs) {
        let EvaluatedWholeStreamCommandArgs { args, input } = self;

        (input, args)
    }
}

#[derive(Getters)]
#[get = "pub"]
pub struct EvaluatedFilterCommandArgs {
    args: EvaluatedCommandArgs,
    #[allow(unused)]
    input: Tagged<Value>,
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
        shell_manager: ShellManager,
        call_info: CallInfo,
        input: Tagged<Value>,
    ) -> EvaluatedFilterCommandArgs {
        EvaluatedFilterCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                shell_manager,
                call_info,
            },
            input,
        }
    }
}

#[derive(Getters, new)]
#[get = "crate"]
pub struct EvaluatedCommandArgs {
    pub host: Arc<Mutex<dyn Host>>,
    pub shell_manager: ShellManager,
    pub call_info: CallInfo,
}

impl EvaluatedCommandArgs {
    pub fn call_args(&self) -> &registry::EvaluatedArgs {
        &self.call_info.args
    }

    pub fn nth(&self, pos: usize) -> Option<&Tagged<Value>> {
        self.call_info.args.nth(pos)
    }

    pub fn expect_nth(&self, pos: usize) -> Result<&Tagged<Value>, ShellError> {
        self.call_info.args.expect_nth(pos)
    }

    pub fn len(&self) -> usize {
        self.call_info.args.len()
    }

    pub fn get(&self, name: &str) -> Option<&Tagged<Value>> {
        self.call_info.args.get(name)
    }

    pub fn slice_from(&self, from: usize) -> Vec<Tagged<Value>> {
        let positional = &self.call_info.args.positional;

        match positional {
            None => vec![],
            Some(list) => list[from..].to_vec(),
        }
    }

    #[allow(unused)]
    pub fn has(&self, name: &str) -> bool {
        self.call_info.args.has(name)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandAction {
    ChangePath(String),
    AddSpanSource(Uuid, SpanSource),
    Exit,
    EnterShell(String),
    EnterValueShell(Tagged<Value>),
    PreviousShell,
    NextShell,
    LeaveShell,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ReturnSuccess {
    Value(Tagged<Value>),
    Action(CommandAction),
}

pub type ReturnValue = Result<ReturnSuccess, ShellError>;

impl From<Tagged<Value>> for ReturnValue {
    fn from(input: Tagged<Value>) -> ReturnValue {
        Ok(ReturnSuccess::Value(input))
    }
}

impl ReturnSuccess {
    pub fn change_cwd(path: String) -> ReturnValue {
        Ok(ReturnSuccess::Action(CommandAction::ChangePath(path)))
    }

    pub fn value(input: impl Into<Tagged<Value>>) -> ReturnValue {
        Ok(ReturnSuccess::Value(input.into()))
    }

    pub fn action(input: CommandAction) -> ReturnValue {
        Ok(ReturnSuccess::Action(input))
    }

    pub fn spanned_value(input: Value, span: Span) -> ReturnValue {
        Ok(ReturnSuccess::Value(Tagged::from_simple_spanned_item(
            input, span,
        )))
    }
}

pub trait WholeStreamCommand: Send + Sync {
    fn name(&self) -> &str;

    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError>;

    fn signature(&self) -> Signature {
        Signature {
            name: self.name().to_string(),
            positional: vec![],
            rest_positional: true,
            named: indexmap::IndexMap::new(),
            is_filter: true,
        }
    }
}

pub trait PerItemCommand: Send + Sync {
    fn name(&self) -> &str;

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        shell_manager: &ShellManager,
        input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError>;

    fn signature(&self) -> Signature {
        Signature {
            name: self.name().to_string(),
            positional: vec![],
            rest_positional: true,
            named: indexmap::IndexMap::new(),
            is_filter: true,
        }
    }
}

pub enum Command {
    WholeStream(Arc<dyn WholeStreamCommand>),
    PerItem(Arc<dyn PerItemCommand>),
}

impl Command {
    pub fn name(&self) -> &str {
        match self {
            Command::WholeStream(command) => command.name(),
            Command::PerItem(command) => command.name(),
        }
    }

    pub fn signature(&self) -> Signature {
        match self {
            Command::WholeStream(command) => command.signature(),
            Command::PerItem(command) => command.signature(),
        }
    }

    pub fn run(&self, args: CommandArgs, registry: &registry::CommandRegistry) -> OutputStream {
        match self {
            Command::WholeStream(command) => match command.run(args, registry) {
                Ok(stream) => stream,
                Err(err) => OutputStream::one(Err(err)),
            },
            Command::PerItem(command) => self.run_helper(command.clone(), args, registry.clone()),
        }
    }

    fn run_helper(
        &self,
        command: Arc<dyn PerItemCommand>,
        args: CommandArgs,
        registry: CommandRegistry,
    ) -> OutputStream {
        let raw_args = RawCommandArgs {
            host: args.host,
            shell_manager: args.shell_manager,
            call_info: args.call_info,
        };

        if raw_args.call_info.has_it_or_block() {
            let out = args
                .input
                .values
                .map(move |x| {
                    let call_info = raw_args
                        .clone()
                        .call_info
                        .evaluate(&registry, &Scope::it_value(x.clone()))
                        .unwrap();
                    match command.run(&call_info, &registry, &raw_args.shell_manager, x) {
                        Ok(o) => o,
                        Err(e) => VecDeque::from(vec![ReturnValue::Err(e)]).to_output_stream(),
                    }
                })
                .flatten();

            out.to_output_stream()
        } else {
            let nothing = Value::nothing().tagged(Tag::unknown());
            let call_info = raw_args
                .clone()
                .call_info
                .evaluate(&registry, &Scope::it_value(nothing.clone()))
                .unwrap();
            // We don't have an $it or block, so just execute what we have

            command
                .run(&call_info, &registry, &raw_args.shell_manager, nothing)?
                .into()
            // let out = match command.run(&call_info, &registry, &raw_args.shell_manager, nothing) {
            //     Ok(o) => o,
            //     Err(e) => VecDeque::from(vec![ReturnValue::Err(e)]),
            // };
            // Ok(out.to_output_stream())
        }
    }
}

#[allow(unused)]
pub struct FnFilterCommand {
    name: String,
    func: fn(EvaluatedFilterCommandArgs) -> Result<OutputStream, ShellError>,
}

impl WholeStreamCommand for FnFilterCommand {
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
            shell_manager,
            call_info,
            input,
        } = args;

        let host: Arc<Mutex<dyn Host>> = host.clone();
        let shell_manager = shell_manager.clone();
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

            let args =
                EvaluatedFilterCommandArgs::new(host.clone(), shell_manager.clone(), call_info, it);

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

pub fn whole_stream_command(command: impl WholeStreamCommand + 'static) -> Arc<Command> {
    Arc::new(Command::WholeStream(Arc::new(command)))
}

pub fn per_item_command(command: impl PerItemCommand + 'static) -> Arc<Command> {
    Arc::new(Command::PerItem(Arc::new(command)))
}

#[allow(unused)]
pub fn filter(
    name: &str,
    func: fn(EvaluatedFilterCommandArgs) -> Result<OutputStream, ShellError>,
) -> Arc<Command> {
    Arc::new(Command::WholeStream(Arc::new(FnFilterCommand {
        name: name.to_string(),
        func,
    })))
}
