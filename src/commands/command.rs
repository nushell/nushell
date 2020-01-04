use crate::context::CommandRegistry;
use crate::deserializer::ConfigDeserializer;
use crate::evaluate::evaluate_args::evaluate_args;
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use nu_errors::ShellError;
use nu_parser::hir;
use nu_protocol::{CallInfo, EvaluatedArgs, ReturnValue, Scope, Signature, Value};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::atomic::AtomicBool;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: hir::Call,
    pub source: Text,
    pub name_tag: Tag,
}

impl UnevaluatedCallInfo {
    pub fn evaluate(
        self,
        registry: &CommandRegistry,
        scope: &Scope,
    ) -> Result<CallInfo, ShellError> {
        let args = evaluate_args(&self.args, registry, scope, &self.source)?;

        Ok(CallInfo {
            args,
            name_tag: self.name_tag,
        })
    }
}

pub trait CallInfoExt {
    fn process<'de, T: Deserialize<'de>>(
        &self,
        shell_manager: &ShellManager,
        callback: fn(T, &RunnablePerItemContext) -> Result<OutputStream, ShellError>,
    ) -> Result<RunnablePerItemArgs<T>, ShellError>;
}

impl CallInfoExt for CallInfo {
    fn process<'de, T: Deserialize<'de>>(
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
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
    pub input: InputStream,
}

#[derive(Getters, Clone)]
#[get = "pub(crate)"]
pub struct RawCommandArgs {
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
}

impl RawCommandArgs {
    pub fn with_input(self, input: Vec<Value>) -> CommandArgs {
        CommandArgs {
            host: self.host,
            ctrl_c: self.ctrl_c,
            shell_manager: self.shell_manager,
            call_info: self.call_info,
            input: input.into(),
        }
    }
}

impl std::fmt::Debug for CommandArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.call_info.fmt(f)
    }
}

impl CommandArgs {
    pub fn evaluate_once(
        self,
        registry: &CommandRegistry,
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

    pub fn evaluate_once_with_scope(
        self,
        registry: &CommandRegistry,
        scope: &Scope,
    ) -> Result<EvaluatedWholeStreamCommandArgs, ShellError> {
        let host = self.host.clone();
        let ctrl_c = self.ctrl_c.clone();
        let shell_manager = self.shell_manager.clone();
        let input = self.input;
        let call_info = self.call_info.evaluate(registry, scope)?;

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
        let mut deserializer = ConfigDeserializer::from_call_info(call_info);

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

pub struct RunnableContext {
    pub input: InputStream,
    pub shell_manager: ShellManager,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub source: Text,
    pub ctrl_c: Arc<AtomicBool>,
    pub commands: CommandRegistry,
    pub name: Tag,
}

impl RunnableContext {
    pub fn get_command(&self, name: &str) -> Result<Option<Arc<Command>>, ShellError> {
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
        host: Arc<parking_lot::Mutex<dyn Host>>,
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

    pub fn parts(self) -> (InputStream, EvaluatedArgs) {
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
        host: Arc<parking_lot::Mutex<dyn Host>>,
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
    pub host: Arc<parking_lot::Mutex<dyn Host>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub shell_manager: ShellManager,
    pub call_info: CallInfo,
}

impl EvaluatedCommandArgs {
    pub fn nth(&self, pos: usize) -> Option<&Value> {
        self.call_info.args.nth(pos)
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.call_info.args.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.call_info.args.has(name)
    }
}

pub trait WholeStreamCommand: Send + Sync {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature {
        Signature::new(self.name()).desc(self.usage()).filter()
    }

    fn usage(&self) -> &str;

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError>;

    fn is_binary(&self) -> bool {
        false
    }
}

pub trait PerItemCommand: Send + Sync {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature {
        Signature::new(self.name()).desc(self.usage()).filter()
    }

    fn usage(&self) -> &str;

    fn run(
        &self,
        call_info: &CallInfo,
        registry: &CommandRegistry,
        raw_args: &RawCommandArgs,
        input: Value,
    ) -> Result<OutputStream, ShellError>;

    fn is_binary(&self) -> bool {
        false
    }
}

pub enum Command {
    WholeStream(Arc<dyn WholeStreamCommand>),
    PerItem(Arc<dyn PerItemCommand>),
}

impl PrettyDebugWithSource for Command {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        match self {
            Command::WholeStream(command) => b::typed(
                "whole stream command",
                b::description(command.name())
                    + b::space()
                    + b::equals()
                    + b::space()
                    + command.signature().pretty_debug(source),
            ),
            Command::PerItem(command) => b::typed(
                "per item command",
                b::description(command.name())
                    + b::space()
                    + b::equals()
                    + b::space()
                    + command.signature().pretty_debug(source),
            ),
        }
    }
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

    pub fn run(&self, args: CommandArgs, registry: &CommandRegistry) -> OutputStream {
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
                    .evaluate(&registry, &Scope::it_value(x.clone()));

                match call_info {
                    Ok(call_info) => match command.run(&call_info, &registry, &raw_args, x) {
                        Ok(o) => o,
                        Err(e) => VecDeque::from(vec![ReturnValue::Err(e)]).to_output_stream(),
                    },
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
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let CommandArgs {
            host,
            ctrl_c,
            shell_manager,
            call_info,
            input,
        } = args;

        let host: Arc<parking_lot::Mutex<dyn Host>> = host.clone();
        let registry: CommandRegistry = registry.clone();
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
                Err(err) => OutputStream::from(vec![Err(err)]).values,
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
