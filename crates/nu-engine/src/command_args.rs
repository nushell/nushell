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
        let ctx = EvaluationContext::from_args(&self);
        let host = self.host.clone();
        let ctrl_c = self.ctrl_c.clone();
        let configs = self.configs.clone();
        let shell_manager = self.shell_manager.clone();
        let input = self.input;
        let call_info = self.call_info.evaluate(&ctx)?;
        let scope = self.scope.clone();

        Ok(EvaluatedWholeStreamCommandArgs::new(
            host,
            ctrl_c,
            configs,
            shell_manager,
            call_info,
            input,
            scope,
        ))
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

    pub fn get_flag<T: FromValue>(&self, name: &str) -> Option<Result<T, ShellError>> {
        self.call_info
            .args
            .get(name)
            .map(|x| FromValue::from_value(x))
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

    pub fn opt<T: FromValue>(&self, pos: usize) -> Option<Result<T, ShellError>> {
        if let Some(v) = self.nth(pos) {
            Some(FromValue::from_value(v))
        } else {
            None
        }
    }
}
