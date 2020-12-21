use crate::deserializer::ConfigDeserializer;
use crate::evaluate::evaluate_args::evaluate_args;
use crate::prelude::*;
use crate::{commands::help::get_help, run_block};
use derive_new::new;
use getset::Getters;
use nu_errors::ShellError;
use nu_protocol::hir::{self, Block};
use nu_protocol::{CallInfo, EvaluatedArgs, ReturnSuccess, Signature, UntaggedValue, Value};
use parking_lot::Mutex;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone)]
pub struct UnevaluatedCallInfo {
    pub args: hir::Call,
    pub name_tag: Tag,
}

impl UnevaluatedCallInfo {
    pub async fn evaluate(self, ctx: &EvaluationContext) -> Result<CallInfo, ShellError> {
        let args = evaluate_args(&self.args, ctx).await?;

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
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
    pub scope: Scope,
    pub input: InputStream,
}

#[derive(Getters, Clone)]
#[get = "pub(crate)"]
pub struct RawCommandArgs {
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub shell_manager: ShellManager,
    pub scope: Scope,
    pub call_info: UnevaluatedCallInfo,
}

impl RawCommandArgs {
    pub fn with_input(self, input: impl Into<InputStream>) -> CommandArgs {
        CommandArgs {
            host: self.host,
            ctrl_c: self.ctrl_c,
            current_errors: self.current_errors,
            shell_manager: self.shell_manager,
            call_info: self.call_info,
            scope: self.scope,
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
    pub async fn evaluate_once(self) -> Result<EvaluatedWholeStreamCommandArgs, ShellError> {
        let ctx = EvaluationContext::from_args(&self);
        let host = self.host.clone();
        let ctrl_c = self.ctrl_c.clone();
        let shell_manager = self.shell_manager.clone();
        let input = self.input;
        let call_info = self.call_info.evaluate(&ctx).await?;
        let scope = self.scope.clone();

        Ok(EvaluatedWholeStreamCommandArgs::new(
            host,
            ctrl_c,
            shell_manager,
            call_info,
            input,
            scope,
        ))
    }

    pub async fn process<'de, T: Deserialize<'de>>(self) -> Result<(T, InputStream), ShellError> {
        let args = self.evaluate_once().await?;
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
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub scope: Scope,
    pub name: Tag,
}

impl RunnableContext {
    pub fn get_command(&self, name: &str) -> Option<Command> {
        self.scope.get_command(name)
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
        scope: Scope,
    ) -> EvaluatedWholeStreamCommandArgs {
        EvaluatedWholeStreamCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                ctrl_c,
                shell_manager,
                call_info,
                scope,
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

#[derive(Getters, new)]
#[get = "pub(crate)"]
pub struct EvaluatedCommandArgs {
    pub host: Arc<parking_lot::Mutex<dyn Host>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub shell_manager: ShellManager,
    pub call_info: CallInfo,
    pub scope: Scope,
}

impl EvaluatedCommandArgs {
    pub fn nth(&self, pos: usize) -> Option<&Value> {
        self.call_info.args.nth(pos)
    }

    /// Get the nth positional argument, error if not possible
    pub fn expect_nth(&self, pos: usize) -> Result<&Value, ShellError> {
        self.call_info
            .args
            .nth(pos)
            .ok_or_else(|| ShellError::unimplemented("Better error: expect_nth"))
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

#[async_trait]
pub trait WholeStreamCommand: Send + Sync {
    fn name(&self) -> &str;

    fn signature(&self) -> Signature {
        Signature::new(self.name()).desc(self.usage()).filter()
    }

    fn usage(&self) -> &str;

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError>;

    fn is_binary(&self) -> bool {
        false
    }

    // Commands that are not meant to be run by users
    fn is_internal(&self) -> bool {
        false
    }

    fn examples(&self) -> Vec<Example> {
        Vec::new()
    }
}

// Custom commands are blocks, so we can use the information in the block to also
// implement a WholeStreamCommand
#[async_trait]
impl WholeStreamCommand for Block {
    fn name(&self) -> &str {
        &self.params.name
    }

    fn signature(&self) -> Signature {
        self.params.clone()
    }

    fn usage(&self) -> &str {
        ""
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let call_info = args.call_info.clone();

        let mut block = self.clone();
        block.set_redirect(call_info.args.external_redirection);

        let ctx = EvaluationContext::from_args(&args);
        let evaluated = call_info.evaluate(&ctx).await?;

        let input = args.input;
        ctx.scope.enter_scope();
        if let Some(args) = evaluated.args.positional {
            // FIXME: do not do this
            for arg in args.into_iter().zip(self.params.positional.iter()) {
                let name = arg.1 .0.name();

                if name.starts_with('$') {
                    ctx.scope.add_var(name, arg.0);
                } else {
                    ctx.scope.add_var(format!("${}", name), arg.0);
                }
            }
        }
        if let Some(args) = evaluated.args.named {
            for named in &block.params.named {
                let name = named.0;
                if let Some(value) = args.get(name) {
                    if name.starts_with('$') {
                        ctx.scope.add_var(name, value.clone());
                    } else {
                        ctx.scope.add_var(format!("${}", name), value.clone());
                    }
                } else if name.starts_with('$') {
                    ctx.scope
                        .add_var(name, UntaggedValue::nothing().into_untagged_value());
                } else {
                    ctx.scope.add_var(
                        format!("${}", name),
                        UntaggedValue::nothing().into_untagged_value(),
                    );
                }
            }
        } else {
            for named in &block.params.named {
                let name = named.0;
                if name.starts_with('$') {
                    ctx.scope
                        .add_var(name, UntaggedValue::nothing().into_untagged_value());
                } else {
                    ctx.scope.add_var(
                        format!("${}", name),
                        UntaggedValue::nothing().into_untagged_value(),
                    );
                }
            }
        }
        let result = run_block(&block, &ctx, input).await;
        ctx.scope.exit_scope();
        result.map(|x| x.to_output_stream())
    }

    fn is_binary(&self) -> bool {
        false
    }

    fn is_internal(&self) -> bool {
        false
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
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

    pub fn examples(&self) -> Vec<Example> {
        self.0.examples()
    }

    pub async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        if args.call_info.switch_present("help") {
            let cl = self.0.clone();
            Ok(OutputStream::one(Ok(ReturnSuccess::Value(
                UntaggedValue::string(get_help(&*cl, &args.scope)).into_value(Tag::unknown()),
            ))))
        } else {
            self.0.run(args).await
        }
    }

    pub fn is_binary(&self) -> bool {
        self.0.is_binary()
    }

    pub fn is_internal(&self) -> bool {
        self.0.is_internal()
    }

    pub fn stream_command(&self) -> &dyn WholeStreamCommand {
        &*self.0
    }
}

pub fn whole_stream_command(command: impl WholeStreamCommand + 'static) -> Command {
    Command(Arc::new(command))
}
