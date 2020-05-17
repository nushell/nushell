use crate::commands::help::get_help;
use crate::context::CommandRegistry;
use crate::deserializer::ConfigDeserializer;
use crate::evaluate::evaluate_args::evaluate_args;
use crate::prelude::*;
use derive_new::new;
use getset::Getters;
use nu_errors::ShellError;
use nu_protocol::hir;
use nu_protocol::{CallInfo, EvaluatedArgs, ReturnSuccess, Scope, Signature, UntaggedValue, Value};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::atomic::AtomicBool;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: hir::Call,
    pub name_tag: Tag,
    pub scope: Scope,
}

impl UnevaluatedCallInfo {
    pub async fn evaluate(self, registry: &CommandRegistry) -> Result<CallInfo, ShellError> {
        let args = evaluate_args(&self.args, registry, &self.scope).await?;

        Ok(CallInfo {
            args,
            name_tag: self.name_tag,
        })
    }

    pub async fn evaluate_with_new_it(
        self,
        registry: &CommandRegistry,
        it: &Value,
    ) -> Result<CallInfo, ShellError> {
        let mut scope = self.scope.clone();
        scope = scope.set_it(it.clone());
        let args = evaluate_args(&self.args, registry, &scope).await?;

        Ok(CallInfo {
            args,
            name_tag: self.name_tag,
        })
    }

    pub fn switch_present(&self, switch: &str) -> bool {
        self.args.switch_preset(switch)
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
    pub fn with_input(self, input: impl Into<InputStream>) -> CommandArgs {
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
    pub async fn evaluate_once(
        self,
        registry: &CommandRegistry,
    ) -> Result<EvaluatedWholeStreamCommandArgs, ShellError> {
        let host = self.host.clone();
        let ctrl_c = self.ctrl_c.clone();
        let shell_manager = self.shell_manager.clone();
        let input = self.input;
        let call_info = self.call_info.evaluate(registry).await?;

        Ok(EvaluatedWholeStreamCommandArgs::new(
            host,
            ctrl_c,
            shell_manager,
            call_info,
            input,
        ))
    }

    pub async fn evaluate_once_with_scope(
        self,
        registry: &CommandRegistry,
        scope: &Scope,
    ) -> Result<EvaluatedWholeStreamCommandArgs, ShellError> {
        let host = self.host.clone();
        let ctrl_c = self.ctrl_c.clone();
        let shell_manager = self.shell_manager.clone();
        let input = self.input;
        let call_info = UnevaluatedCallInfo {
            name_tag: self.call_info.name_tag,
            args: self.call_info.args,
            scope: scope.clone(),
        };
        let call_info = call_info.evaluate(registry).await?;

        Ok(EvaluatedWholeStreamCommandArgs::new(
            host,
            ctrl_c,
            shell_manager,
            call_info,
            input,
        ))
    }

    pub async fn process<'de, T: Deserialize<'de>>(
        self,
        registry: &CommandRegistry,
    ) -> Result<(T, InputStream), ShellError> {
        let args = self.evaluate_once(registry).await?;
        let call_info = args.call_info.clone();

        let mut deserializer = ConfigDeserializer::from_call_info(call_info);

        Ok((T::deserialize(&mut deserializer)?, args.input))
    }
}

pub struct RunnableContext {
    pub input: InputStream,
    pub shell_manager: ShellManager,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub registry: CommandRegistry,
    pub name: Tag,
}

impl RunnableContext {
    pub fn get_command(&self, name: &str) -> Option<Command> {
        self.registry.get_command(name)
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

    /// Get the nth positional argument, error if not possible
    pub fn expect_nth(&self, pos: usize) -> Result<&Value, ShellError> {
        match self.call_info.args.nth(pos) {
            None => Err(ShellError::unimplemented("Better error: expect_nth")),
            Some(item) => Ok(item),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.call_info.args.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.call_info.args.has(name)
    }
}

pub struct Example {
    pub example: &'static str,
    pub description: &'static str,
    pub result: Option<Vec<Value>>,
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

    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }
}

#[derive(Clone)]
pub struct Command(Arc<dyn WholeStreamCommand>);

impl PrettyDebugWithSource for Command {
    fn pretty_debug(&self, source: &str) -> DebugDocBuilder {
        b::typed(
            "whole stream command",
            b::description(self.name())
                + b::space()
                + b::equals()
                + b::space()
                + self.signature().pretty_debug(source),
        )
    }
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command({})", self.name())
    }
}

impl Command {
    pub fn name(&self) -> &str {
        self.0.name()
    }

    pub fn signature(&self) -> Signature {
        self.0.signature()
    }

    pub fn usage(&self) -> &str {
        self.0.usage()
    }

    pub fn run(&self, args: CommandArgs, registry: &CommandRegistry) -> OutputStream {
        if args.call_info.switch_present("help") {
            let cl = self.0.clone();
            let registry = registry.clone();
            let stream = async_stream! {
                yield Ok(ReturnSuccess::Value(UntaggedValue::string(get_help(&*cl, &registry)).into_value(Tag::unknown())));
            };
            stream.to_output_stream()
        } else {
            match self.0.run(args, registry) {
                Ok(stream) => stream,
                Err(err) => OutputStream::one(Err(err)),
            }
        }
    }

    pub fn is_binary(&self) -> bool {
        self.0.is_binary()
    }

    pub fn stream_command(&self) -> &dyn WholeStreamCommand {
        &*self.0
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
            mut input,
        } = args;

        let host: Arc<parking_lot::Mutex<dyn Host>> = host.clone();
        let registry: CommandRegistry = registry.clone();
        let func = self.func;

        let stream = async_stream! {
            while let Some(it) = input.next().await {
                let registry = registry.clone();
                let call_info = match call_info.clone().evaluate_with_new_it(&registry, &it).await {
                    Err(err) => { yield Err(err); return; },
                    Ok(args) => args,
                };

                let args = EvaluatedFilterCommandArgs::new(
                    host.clone(),
                    ctrl_c.clone(),
                    shell_manager.clone(),
                    call_info,
                );

                match func(args) {
                    Err(err) => yield Err(err),
                    Ok(mut stream) => {
                        while let Some(value) = stream.values.next().await {
                            yield value;
                        }
                    }
                }
            }
        };

        Ok(stream.to_output_stream())
    }
}

pub fn whole_stream_command(command: impl WholeStreamCommand + 'static) -> Command {
    Command(Arc::new(command))
}
