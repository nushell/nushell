use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use nu_engine::get_eval_block_with_early_return;
use nu_protocol::{
    ast::Call,
    engine::{Closure, EngineState, Stack},
    Config, IntoSpanned, PipelineData, PluginIdentity, ShellError, Span, Spanned, Value,
};

/// Object safe trait for abstracting operations required of the plugin context.
pub(crate) trait PluginExecutionContext: Send + Sync {
    /// The [Span] for the command execution (`call.head`)
    fn command_span(&self) -> Span;
    /// The name of the command being executed
    fn command_name(&self) -> &str;
    /// The interrupt signal, if present
    fn ctrlc(&self) -> Option<&Arc<AtomicBool>>;
    /// Get engine configuration
    fn get_config(&self) -> Result<Config, ShellError>;
    /// Get plugin configuration
    fn get_plugin_config(&self) -> Result<Option<Value>, ShellError>;
    /// Get an environment variable from `$env`
    fn get_env_var(&self, name: &str) -> Result<Option<Value>, ShellError>;
    /// Get all environment variables
    fn get_env_vars(&self) -> Result<HashMap<String, Value>, ShellError>;
    // Get current working directory
    fn get_current_dir(&self) -> Result<Spanned<String>, ShellError>;
    /// Evaluate a closure passed to the plugin
    fn eval_closure(
        &self,
        closure: Spanned<Closure>,
        positional: Vec<Value>,
        input: PipelineData,
        redirect_stdout: bool,
        redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError>;
}

/// The execution context of a plugin command.
pub(crate) struct PluginExecutionCommandContext {
    identity: Arc<PluginIdentity>,
    engine_state: EngineState,
    stack: Stack,
    call: Call,
}

impl PluginExecutionCommandContext {
    pub fn new(
        identity: Arc<PluginIdentity>,
        engine_state: &EngineState,
        stack: &Stack,
        call: &Call,
    ) -> PluginExecutionCommandContext {
        PluginExecutionCommandContext {
            identity,
            engine_state: engine_state.clone(),
            stack: stack.clone(),
            call: call.clone(),
        }
    }
}

impl PluginExecutionContext for PluginExecutionCommandContext {
    fn command_span(&self) -> Span {
        self.call.head
    }

    fn command_name(&self) -> &str {
        self.engine_state.get_decl(self.call.decl_id).name()
    }

    fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        self.engine_state.ctrlc.as_ref()
    }

    fn get_config(&self) -> Result<Config, ShellError> {
        Ok(nu_engine::get_config(&self.engine_state, &self.stack))
    }

    fn get_plugin_config(&self) -> Result<Option<Value>, ShellError> {
        // Fetch the configuration for a plugin
        //
        // The `plugin` must match the registered name of a plugin.  For
        // `register nu_plugin_example` the plugin config lookup uses `"example"`
        Ok(self
            .get_config()?
            .plugins
            .get(self.identity.name())
            .cloned()
            .map(|value| {
                let span = value.span();
                match value {
                    Value::Closure { val, .. } => {
                        let input = PipelineData::Empty;

                        let block = self.engine_state.get_block(val.block_id).clone();
                        let mut stack = self.stack.captures_to_stack(val.captures);

                        let eval_block_with_early_return =
                            get_eval_block_with_early_return(&self.engine_state);

                        match eval_block_with_early_return(
                            &self.engine_state,
                            &mut stack,
                            &block,
                            input,
                            false,
                            false,
                        ) {
                            Ok(v) => v.into_value(span),
                            Err(e) => Value::error(e, self.call.head),
                        }
                    }
                    _ => value.clone(),
                }
            }))
    }

    fn get_env_var(&self, name: &str) -> Result<Option<Value>, ShellError> {
        Ok(self.stack.get_env_var(&self.engine_state, name))
    }

    fn get_env_vars(&self) -> Result<HashMap<String, Value>, ShellError> {
        Ok(self.stack.get_env_vars(&self.engine_state))
    }

    fn get_current_dir(&self) -> Result<Spanned<String>, ShellError> {
        let cwd = nu_engine::env::current_dir_str(&self.engine_state, &self.stack)?;
        // The span is not really used, so just give it call.head
        Ok(cwd.into_spanned(self.call.head))
    }

    fn eval_closure(
        &self,
        closure: Spanned<Closure>,
        positional: Vec<Value>,
        input: PipelineData,
        redirect_stdout: bool,
        redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError> {
        let block = self
            .engine_state
            .try_get_block(closure.item.block_id)
            .ok_or_else(|| ShellError::GenericError {
                error: "Plugin misbehaving".into(),
                msg: format!(
                    "Tried to evaluate unknown block id: {}",
                    closure.item.block_id
                ),
                span: Some(closure.span),
                help: None,
                inner: vec![],
            })?;

        let mut stack = self.stack.captures_to_stack(closure.item.captures);

        // Set up the positional arguments
        for (idx, value) in positional.into_iter().enumerate() {
            if let Some(arg) = block.signature.get_positional(idx) {
                if let Some(var_id) = arg.var_id {
                    stack.add_var(var_id, value);
                } else {
                    return Err(ShellError::NushellFailedSpanned {
                        msg: "Error while evaluating closure from plugin".into(),
                        label: "closure argument missing var_id".into(),
                        span: closure.span,
                    });
                }
            }
        }

        let eval_block_with_early_return = get_eval_block_with_early_return(&self.engine_state);

        eval_block_with_early_return(
            &self.engine_state,
            &mut stack,
            block,
            input,
            redirect_stdout,
            redirect_stderr,
        )
    }
}

/// A bogus execution context for testing that doesn't really implement anything properly
#[cfg(test)]
pub(crate) struct PluginExecutionBogusContext;

#[cfg(test)]
impl PluginExecutionContext for PluginExecutionBogusContext {
    fn command_span(&self) -> Span {
        Span::test_data()
    }

    fn command_name(&self) -> &str {
        "bogus"
    }

    fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        None
    }

    fn get_config(&self) -> Result<Config, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "get_config not implemented on bogus".into(),
        })
    }

    fn get_plugin_config(&self) -> Result<Option<Value>, ShellError> {
        Ok(None)
    }

    fn get_env_var(&self, _name: &str) -> Result<Option<Value>, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "get_env_var not implemented on bogus".into(),
        })
    }

    fn get_env_vars(&self) -> Result<HashMap<String, Value>, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "get_env_vars not implemented on bogus".into(),
        })
    }

    fn get_current_dir(&self) -> Result<Spanned<String>, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "get_current_dir not implemented on bogus".into(),
        })
    }

    fn eval_closure(
        &self,
        _closure: Spanned<Closure>,
        _positional: Vec<Value>,
        _input: PipelineData,
        _redirect_stdout: bool,
        _redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "eval_closure not implemented on bogus".into(),
        })
    }
}
