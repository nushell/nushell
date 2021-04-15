use crate::deserializer::ConfigDeserializer;
use crate::env::host::Host;
use crate::evaluate::scope::Scope;
use crate::evaluation_context::EvaluationContext;
use crate::shell::shell_manager::ShellManager;
use crate::FromValue;
use crate::{call_info::UnevaluatedCallInfo, config_holder::ConfigHolder};
use derive_new::new;
use getset::Getters;
use nu_errors::ShellError;
use nu_protocol::EvaluatedArgs;
use nu_protocol::{CallInfo, Value};
use nu_source::Tag;
use nu_stream::InputStream;
use parking_lot::Mutex;
use serde::Deserialize;
use std::ops::Deref;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Getters)]
#[get = "pub"]
pub struct CommandArgs {
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub configs: Arc<Mutex<ConfigHolder>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub shell_manager: ShellManager,
    pub call_info: UnevaluatedCallInfo,
    pub scope: Scope,
    pub input: InputStream,
}

pub type RunnableContext = CommandArgs;

#[derive(Clone)]
pub struct RunnableContextWithoutInput {
    pub shell_manager: ShellManager,
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub call_info: UnevaluatedCallInfo,
    pub configs: Arc<Mutex<ConfigHolder>>,
    pub scope: Scope,
    pub name: Tag,
}

impl RunnableContextWithoutInput {
    pub fn with_input(self, input: InputStream) -> CommandArgs {
        CommandArgs {
            shell_manager: self.shell_manager,
            host: self.host,
            current_errors: self.current_errors,
            ctrl_c: self.ctrl_c,
            call_info: self.call_info,
            configs: self.configs,
            scope: self.scope,
            input,
        }
    }
}

#[derive(Getters, Clone)]
#[get = "pub"]
pub struct RawCommandArgs {
    pub host: Arc<parking_lot::Mutex<Box<dyn Host>>>,
    pub ctrl_c: Arc<AtomicBool>,
    pub current_errors: Arc<Mutex<Vec<ShellError>>>,
    pub configs: Arc<Mutex<ConfigHolder>>,
    pub shell_manager: ShellManager,
    pub scope: Scope,
    pub call_info: UnevaluatedCallInfo,
}

impl RawCommandArgs {
    pub fn with_input(self, input: impl Into<InputStream>) -> CommandArgs {
        CommandArgs {
            host: self.host,
            ctrl_c: self.ctrl_c,
            configs: self.configs,
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
    pub fn evaluate_once(self) -> Result<EvaluatedWholeStreamCommandArgs, ShellError> {
        let ctx = EvaluationContext::new(
            self.scope,
            self.host,
            self.current_errors,
            self.ctrl_c,
            self.configs,
            self.shell_manager,
            Arc::new(Mutex::new(std::collections::HashMap::new())),
        );

        let input = self.input;
        let call_info = self.call_info.evaluate(&ctx)?;

        Ok(EvaluatedWholeStreamCommandArgs::new(
            ctx.host,
            ctx.ctrl_c,
            ctx.configs,
            ctx.shell_manager,
            call_info,
            input,
            ctx.scope,
        ))
    }

    pub fn split(self) -> (InputStream, RunnableContextWithoutInput) {
        let new_context = RunnableContextWithoutInput {
            shell_manager: self.shell_manager,
            host: self.host,
            ctrl_c: self.ctrl_c,
            configs: self.configs,
            name: self.call_info.name_tag.clone(),
            call_info: self.call_info,
            current_errors: self.current_errors,
            scope: self.scope,
        };

        (self.input, new_context)
    }

    pub fn extract<T>(
        self,
        f: impl FnOnce(&EvaluatedCommandArgs) -> Result<T, ShellError>,
    ) -> Result<(T, InputStream), ShellError> {
        let evaluated_args = self.evaluate_once()?;

        Ok((f(&evaluated_args.args)?, evaluated_args.input))
    }

    pub fn process<'de, T: Deserialize<'de>>(self) -> Result<(T, InputStream), ShellError> {
        let args = self.evaluate_once()?;
        let call_info = args.call_info.clone();

        let mut deserializer = ConfigDeserializer::from_call_info(call_info);

        Ok((T::deserialize(&mut deserializer)?, args.input))
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
        configs: Arc<Mutex<ConfigHolder>>,
        shell_manager: ShellManager,
        call_info: CallInfo,
        input: impl Into<InputStream>,
        scope: Scope,
    ) -> EvaluatedWholeStreamCommandArgs {
        EvaluatedWholeStreamCommandArgs {
            args: EvaluatedCommandArgs {
                host,
                ctrl_c,
                configs,
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
    pub configs: Arc<Mutex<ConfigHolder>>,
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

    pub fn get_flag<T: FromValue>(&self, name: &str) -> Result<Option<T>, ShellError> {
        if let Some(arg) = self.call_info.args.get(name) {
            FromValue::from_value(arg).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn req_named<T: FromValue>(&self, name: &str) -> Result<T, ShellError> {
        self.call_info
            .args
            .expect_get(name)
            .and_then(|x| FromValue::from_value(x))
    }

    pub fn has_flag(&self, name: &str) -> bool {
        self.call_info.args.has(name)
    }

    pub fn req<T: FromValue>(&self, pos: usize) -> Result<T, ShellError> {
        if let Some(v) = self.nth(pos) {
            FromValue::from_value(v)
        } else {
            Err(ShellError::labeled_error(
                "Position beyond end of command arguments",
                "can't access beyond end of command arguments",
                self.call_info.name_tag.span,
            ))
        }
    }

    pub fn opt<T: FromValue>(&self, pos: usize) -> Result<Option<T>, ShellError> {
        if let Some(v) = self.nth(pos) {
            FromValue::from_value(v).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn rest_args<T: FromValue>(&self) -> Result<Vec<T>, ShellError> {
        self.rest(0)
    }

    pub fn rest<T: FromValue>(&self, starting_pos: usize) -> Result<Vec<T>, ShellError> {
        let mut output = vec![];

        for val in self.call_info.args.positional_iter().skip(starting_pos) {
            output.push(FromValue::from_value(val)?);
        }

        Ok(output)
    }
}
