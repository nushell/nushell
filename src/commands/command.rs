use crate::data::Value;
use crate::errors::ShellError;
use crate::evaluate::Scope;
use crate::parser::hir;
use crate::parser::{registry, ConfigDeserializer};
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: hir::Call,
    pub source: Text,
    pub name_tag: Tag,
}

impl FormatDebug for UnevaluatedCallInfo {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
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
            name_tag: self.name_tag,
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CallInfo {
    pub args: registry::EvaluatedArgs,
    pub name_tag: Tag,
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
                name: self.name_tag.clone(),
            },
            callback,
        })
    }
}

#[derive(Getters)]
#[get = "pub(crate)"]
pub struct CommandArgs {
    pub host: Arc<Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
    pub input: InputStream,
}

#[derive(Getters, Clone)]
#[get = "pub(crate)"]
pub struct RawCommandArgs {
    pub host: Arc<Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
}

impl RawCommandArgs {
    pub fn with_input(self, input: Vec<Tagged<Value>>) -> CommandArgs {
        CommandArgs {
            host: self.host,
            ctrl_c: self.ctrl_c,
            shell_manager: self.shell_manager,
            call_info: self.call_info,
            input: input.into(),
        }
    }

    pub fn source(&self) -> Text {
        self.call_info.source.clone()
    }
}

impl std::fmt::Debug for CommandArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.call_info.fmt(f)
    }
}

impl FormatDebug for CommandArgs {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        self.call_info.fmt_debug(f, source)
    }
}

impl CommandArgs {
    pub fn evaluate_once(
        self,
        registry: &registry::CommandRegistry,
    ) -> Result<EvaluatedWholeStreamCommandArgs, ShellError> {
        let host = self.host.clone();
        let ctrl_c = self.ctrl_c.clone();
        let shell_manager = self.shell_manager.clone();
        let input = self.input;
        let call_info = self.call_info.evaluate(registry, &Scope::empty())?;

        Ok(EvaluatedWholeStreamCommandArgs::new(
            host,
            ctrl_c,
            shell_manager,
            call_info,
            input,
        ))
    }

    pub fn source(&self) -> Text {
        self.call_info.source.clone()
    }

    pub fn process<'de, T: Deserialize<'de>, O: ToOutputStream>(
        self,
        registry: &CommandRegistry,
        callback: fn(T, RunnableContext) -> Result<O, ShellError>,
    ) -> Result<RunnableArgs<T, O>, ShellError> {
        let shell_manager = self.shell_manager.clone();
        let host = self.host.clone();
        let source = self.source();
        let ctrl_c = self.ctrl_c.clone();
        let args = self.evaluate_once(registry)?;
        let call_info = args.call_info.clone();
        let (input, args) = args.split();
        let name_tag = args.call_info.name_tag;
        let mut deserializer = ConfigDeserializer::from_call_info(call_info);

        Ok(RunnableArgs {
            args: T::deserialize(&mut deserializer)?,
            context: RunnableContext {
                input,
                commands: registry.clone(),
                source,
                shell_manager,
                name: name_tag,
                host,
                ctrl_c,
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
            ctrl_c: self.ctrl_c.clone(),
            shell_manager: self.shell_manager.clone(),
            call_info: self.call_info.clone(),
        };

        let shell_manager = self.shell_manager.clone();
        let host = self.host.clone();
        let source = self.source();
        let ctrl_c = self.ctrl_c.clone();
        let args = self.evaluate_once(registry)?;
        let call_info = args.call_info.clone();

        let (input, args) = args.split();
        let name_tag = args.call_info.name_tag;
        let mut deserializer = ConfigDeserializer::from_call_info(call_info.clone());

        Ok(RunnableRawArgs {
            args: T::deserialize(&mut deserializer)?,
            context: RunnableContext {
                input,
                commands: registry.clone(),
                source,
                shell_manager,
                name: name_tag,
                host,
                ctrl_c,
            },
            raw_args,
            callback,
        })
    }
}

pub struct RunnablePerItemContext {
    pub shell_manager: ShellManager,
    pub name: Tag,
}

impl RunnablePerItemContext {
    pub fn cwd(&self) -> PathBuf {
        PathBuf::from(self.shell_manager.path())
    }
}

pub struct RunnableContext {
    pub input: InputStream,
    pub shell_manager: ShellManager,
    pub host: Arc<Mutex<Box<dyn Host>>>,
    pub source: Text,
    pub ctrl_c: Arc<AtomicBool>,
    pub commands: CommandRegistry,
    pub name: Tag,
}

impl RunnableContext {
    pub fn get_command(&self, name: &str) -> Option<Arc<Command>> {
        self.commands.get_command(name)
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

pub struct RunnableArgs<T, O: ToOutputStream> {
    args: T,
    context: RunnableContext,
    callback: fn(T, RunnableContext) -> Result<O, ShellError>,
}

impl<T, O: ToOutputStream> RunnableArgs<T, O> {
    pub fn run(self) -> Result<OutputStream, ShellError> {
        (self.callback)(self.args, self.context).map(|v| v.to_output_stream())
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
        ctrl_c: Arc<AtomicBool>,
        shell_manager: ShellManager,
        call_info: CallInfo,
        input: impl Into<InputStream>,
    ) -> EvaluatedWholeStreamCommandArgs {
        EvaluatedWholeStreamCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                ctrl_c,
                shell_manager,
                call_info,
            },
            input: input.into(),
        }
    }

    pub fn name_tag(&self) -> Tag {
        self.args.call_info.name_tag.clone()
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
        ctrl_c: Arc<AtomicBool>,
        shell_manager: ShellManager,
        call_info: CallInfo,
    ) -> EvaluatedFilterCommandArgs {
        EvaluatedFilterCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                ctrl_c,
                shell_manager,
                call_info,
            },
        }
    }
}

#[derive(Getters, new)]
#[get = "pub(crate)"]
pub struct EvaluatedCommandArgs {
    pub host: Arc<Mutex<dyn Host>>,
    pub ctrl_c: Arc<AtomicBool>,
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

    pub fn has(&self, name: &str) -> bool {
        self.call_info.args.has(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandAction {
    ChangePath(String),
    Exit,
    Error(ShellError),
    EnterShell(String),
    EnterValueShell(Tagged<Value>),
    EnterHelpShell(Tagged<Value>),
    PreviousShell,
    NextShell,
    LeaveShell,
}

impl FormatDebug for CommandAction {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        match self {
            CommandAction::ChangePath(s) => write!(f, "action:change-path={}", s),
            CommandAction::Exit => write!(f, "action:exit"),
            CommandAction::Error(_) => write!(f, "action:error"),
            CommandAction::EnterShell(s) => write!(f, "action:enter-shell={}", s),
            CommandAction::EnterValueShell(t) => {
                write!(f, "action:enter-value-shell={}", t.debug(source))
            }
            CommandAction::EnterHelpShell(t) => {
                write!(f, "action:enter-help-shell={}", t.debug(source))
            }
            CommandAction::PreviousShell => write!(f, "action:previous-shell"),
            CommandAction::NextShell => write!(f, "action:next-shell"),
            CommandAction::LeaveShell => write!(f, "action:leave-shell"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReturnSuccess {
    Value(Tagged<Value>),
    DebugValue(Tagged<Value>),
    Action(CommandAction),
}

pub type ReturnValue = Result<ReturnSuccess, ShellError>;

impl FormatDebug for ReturnValue {
    fn fmt_debug(&self, f: &mut DebugFormatter, source: &str) -> fmt::Result {
        match self {
            Err(err) => write!(f, "{}", err.debug(source)),
            Ok(ReturnSuccess::Value(v)) => write!(f, "{}", v.debug(source)),
            Ok(ReturnSuccess::DebugValue(v)) => v.fmt_debug(f, source),
            Ok(ReturnSuccess::Action(a)) => write!(f, "{}", a.debug(source)),
        }
    }
}

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

    pub fn debug_value(input: impl Into<Tagged<Value>>) -> ReturnValue {
        Ok(ReturnSuccess::DebugValue(input.into()))
    }

    pub fn action(input: CommandAction) -> ReturnValue {
        Ok(ReturnSuccess::Action(input))
    }
}

pub trait WholeStreamCommand: Send + Sync {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature {
        Signature {
            name: self.name().to_string(),
            usage: self.usage().to_string(),
            positional: vec![],
            rest_positional: None,
            named: indexmap::IndexMap::new(),
            is_filter: true,
        }
    }

    fn usage(&self) -> &str;

    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError>;

    fn is_binary(&self) -> bool {
        false
    }
}

pub trait PerItemCommand: Send + Sync {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature {
        Signature {
            name: self.name().to_string(),
            usage: self.usage().to_string(),
            positional: vec![],
            rest_positional: None,
            named: indexmap::IndexMap::new(),
            is_filter: true,
        }
    }

    fn usage(&self) -> &str;

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        input: Tagged<Value>,
    ) -> Result<OutputStream, ShellError>;

    fn is_binary(&self) -> bool {
        false
    }
}

pub enum Command {
    WholeStream(Arc<dyn WholeStreamCommand>),
    PerItem(Arc<dyn PerItemCommand>),
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::WholeStream(command) => write!(f, "WholeStream({})", command.name()),
            Command::PerItem(command) => write!(f, "PerItem({})", command.name()),
        }
    }
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

    pub fn usage(&self) -> &str {
        match self {
            Command::WholeStream(command) => command.usage(),
            Command::PerItem(command) => command.usage(),
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
            ctrl_c: args.ctrl_c,
            shell_manager: args.shell_manager,
            call_info: args.call_info,
        };

        let out = args
            .input
            .values
            .map(move |x| {
                let call_info = raw_args
                    .clone()
                    .call_info
                    .evaluate(&registry, &Scope::it_value(x.clone()))
                    .unwrap();
                match command.run(&call_info, &registry, &raw_args, x) {
                    Ok(o) => o,
                    Err(e) => VecDeque::from(vec![ReturnValue::Err(e)]).to_output_stream(),
                }
            })
            .flatten();

        out.to_output_stream()
    }

    pub fn is_binary(&self) -> bool {
        match self {
            Command::WholeStream(command) => command.is_binary(),
            Command::PerItem(command) => command.is_binary(),
        }
    }
}

pub struct FnFilterCommand {
    name: String,
    func: fn(EvaluatedFilterCommandArgs) -> Result<OutputStream, ShellError>,
}

impl WholeStreamCommand for FnFilterCommand {
    fn name(&self) -> &str {
        &self.name
    }

    fn usage(&self) -> &str {
        "usage"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &registry::CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let CommandArgs {
            host,
            ctrl_c,
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
            let call_info = match call_info.clone().evaluate(&registry, &Scope::it_value(it)) {
                Err(err) => return OutputStream::from(vec![Err(err)]).values,
                Ok(args) => args,
            };

            let args = EvaluatedFilterCommandArgs::new(
                host.clone(),
                ctrl_c.clone(),
                shell_manager.clone(),
                call_info,
            );

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
