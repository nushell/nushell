use crate::call_info::UnevaluatedCallInfo;
use crate::deserializer::ConfigDeserializer;
use crate::env::host::Host;
use crate::evaluate::scope::Scope;
use crate::evaluation_context::EvaluationContext;
use crate::shell::shell_manager::ShellManager;
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
