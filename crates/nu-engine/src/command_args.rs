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
    pub context: EvaluationContext,
    pub call_info: UnevaluatedCallInfo,
    pub input: InputStream,
}

impl CommandArgs {
    pub fn scope(&self) -> &Scope {
        &self.context.scope
    }

    pub fn host(&self) -> Arc<parking_lot::Mutex<Box<dyn Host>>> {
        self.context.host.clone()
    }

    pub fn current_errors(&self) -> Arc<Mutex<Vec<ShellError>>> {
        self.context.current_errors.clone()
    }

    pub fn ctrl_c(&self) -> Arc<AtomicBool> {
        self.context.ctrl_c.clone()
    }

    pub fn configs(&self) -> Arc<Mutex<ConfigHolder>> {
        self.context.configs.clone()
    }

    pub fn shell_manager(&self) -> ShellManager {
        self.context.shell_manager.clone()
    }
}

pub type RunnableContext = CommandArgs;

impl std::fmt::Debug for CommandArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.call_info.fmt(f)
    }
}

impl CommandArgs {
    pub fn evaluate_once(self) -> Result<EvaluatedCommandArgs, ShellError> {
        let ctx = self.context.clone();

        let input = self.input;
        let call_info = self.call_info.evaluate(&ctx)?;

        Ok(EvaluatedCommandArgs::new(ctx, call_info, input))
    }

    pub fn extract<T>(
        self,
        f: impl FnOnce(&EvaluatedCommandArgsWithoutInput) -> Result<T, ShellError>,
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

pub struct EvaluatedCommandArgs {
    pub args: EvaluatedCommandArgsWithoutInput,
    pub input: InputStream,
}

impl Deref for EvaluatedCommandArgs {
    type Target = EvaluatedCommandArgsWithoutInput;
    fn deref(&self) -> &Self::Target {
        &self.args
    }
}

impl EvaluatedCommandArgs {
    pub fn new(
        context: EvaluationContext,
        call_info: CallInfo,
        input: impl Into<InputStream>,
    ) -> EvaluatedCommandArgs {
        EvaluatedCommandArgs {
            args: EvaluatedCommandArgsWithoutInput { context, call_info },
            input: input.into(),
        }
    }

    pub fn name_tag(&self) -> Tag {
        self.args.call_info.name_tag.clone()
    }

    pub fn parts(self) -> (InputStream, EvaluatedArgs) {
        let EvaluatedCommandArgs { args, input } = self;

        (input, args.call_info.args)
    }

    pub fn split(self) -> (InputStream, EvaluatedCommandArgsWithoutInput) {
        let EvaluatedCommandArgs { args, input } = self;

        (input, args)
    }
}

#[derive(Getters, new)]
#[get = "pub(crate)"]
pub struct EvaluatedCommandArgsWithoutInput {
    pub context: EvaluationContext,
    pub call_info: CallInfo,
}

impl EvaluatedCommandArgsWithoutInput {
    pub fn nth(&self, pos: usize) -> Option<&Value> {
        self.call_info.args.nth(pos)
    }

    pub fn scope(&self) -> Scope {
        self.context.scope.clone()
    }

    pub fn configs(&self) -> Arc<Mutex<ConfigHolder>> {
        self.context.configs.clone()
    }

    pub fn host(&self) -> Arc<parking_lot::Mutex<Box<dyn Host>>> {
        self.context.host.clone()
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

    pub fn rest_with_minimum<T: FromValue>(
        &self,
        pos: usize,
        count: usize,
    ) -> Result<Vec<T>, ShellError> {
        let mut output = vec![];
        for i in pos..pos + count {
            output.push(self.req(i)?);
        }
        output.extend(self.rest(pos + count)?);

        Ok(output)
    }
}
