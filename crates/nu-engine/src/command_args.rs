use crate::evaluate::scope::Scope;
use crate::evaluation_context::EvaluationContext;
use crate::shell::shell_manager::ShellManager;
use crate::FromValue;
use crate::{call_info::UnevaluatedCallInfo, config_holder::ConfigHolder};
use crate::{env::host::Host, evaluate_baseline_expr};
use getset::Getters;
use nu_errors::ShellError;
use nu_protocol::hir::SpannedExpression;
use nu_source::Tag;
use nu_stream::InputStream;
use parking_lot::Mutex;
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
        self.context.host().clone()
    }

    pub fn current_errors(&self) -> Arc<Mutex<Vec<ShellError>>> {
        self.context.current_errors().clone()
    }

    pub fn ctrl_c(&self) -> Arc<AtomicBool> {
        self.context.ctrl_c().clone()
    }

    pub fn configs(&self) -> Arc<Mutex<ConfigHolder>> {
        self.context.configs().clone()
    }

    pub fn shell_manager(&self) -> ShellManager {
        self.context.shell_manager().clone()
    }

    pub fn nth(&self, pos: usize) -> Option<&SpannedExpression> {
        if let Some(positional) = &self.call_info.args.positional {
            positional.get(pos)
        } else {
            None
        }
    }

    pub fn req<T: FromValue>(&self, pos: usize) -> Result<T, ShellError> {
        if let Some(expr) = self.nth(pos) {
            let result = evaluate_baseline_expr(expr, &self.context)?;
            FromValue::from_value(&result)
        } else {
            Err(ShellError::labeled_error(
                "Position beyond end of command arguments",
                "can't access beyond end of command arguments",
                self.call_info.name_tag.span,
            ))
        }
    }

    pub fn req_named<T: FromValue>(&self, name: &str) -> Result<T, ShellError> {
        match self.get_flag(name)? {
            Some(v) => Ok(v),
            None => Err(ShellError::labeled_error(
                "Missing flag",
                format!("expected {} flag", name),
                &self.call_info.name_tag,
            )),
        }
    }

    pub fn has_flag(&self, name: &str) -> bool {
        self.call_info.args.switch_preset(name)
    }

    pub fn get_flag<T: FromValue>(&self, name: &str) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.call_info.args.get_flag(name) {
            let result = evaluate_baseline_expr(expr, &self.context)?;
            FromValue::from_value(&result).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn opt<T: FromValue>(&self, pos: usize) -> Result<Option<T>, ShellError> {
        if let Some(expr) = self.nth(pos) {
            let result = evaluate_baseline_expr(expr, &self.context)?;
            FromValue::from_value(&result).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn rest<T: FromValue>(&self, starting_pos: usize) -> Result<Vec<T>, ShellError> {
        let mut output = vec![];

        if let Some(positional) = &self.call_info.args.positional {
            for expr in positional.iter().skip(starting_pos) {
                let result = evaluate_baseline_expr(expr, &self.context)?;
                output.push(FromValue::from_value(&result)?);
            }
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

    pub fn name_tag(&self) -> Tag {
        self.call_info.name_tag.clone()
    }
}

pub type RunnableContext = CommandArgs;

impl std::fmt::Debug for CommandArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.call_info.fmt(f)
    }
}
