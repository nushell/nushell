use crate::util::MutableCow;
use nu_engine::{get_eval_block_with_early_return, get_full_help, ClosureEvalOnce};
use nu_protocol::{
    ast::Call,
    engine::{Closure, EngineState, Redirection, Stack},
    Config, IntoSpanned, OutDest, PipelineData, PluginIdentity, ShellError, Span, Spanned, Value,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
};

/// Object safe trait for abstracting operations required of the plugin context.
///
/// This is not a public API.
#[doc(hidden)]
pub trait PluginExecutionContext: Send + Sync {
    /// A span pointing to the command being executed
    fn span(&self) -> Span;
    /// The interrupt signal, if present
    fn ctrlc(&self) -> Option<&Arc<AtomicBool>>;
    /// The pipeline externals state, for tracking the foreground process group, if present
    fn pipeline_externals_state(&self) -> Option<&Arc<(AtomicU32, AtomicU32)>>;
    /// Get engine configuration
    fn get_config(&self) -> Result<Config, ShellError>;
    /// Get plugin configuration
    fn get_plugin_config(&self) -> Result<Option<Value>, ShellError>;
    /// Get an environment variable from `$env`
    fn get_env_var(&self, name: &str) -> Result<Option<Value>, ShellError>;
    /// Get all environment variables
    fn get_env_vars(&self) -> Result<HashMap<String, Value>, ShellError>;
    /// Get current working directory
    fn get_current_dir(&self) -> Result<Spanned<String>, ShellError>;
    /// Set an environment variable
    fn add_env_var(&mut self, name: String, value: Value) -> Result<(), ShellError>;
    /// Get help for the current command
    fn get_help(&self) -> Result<Spanned<String>, ShellError>;
    /// Get the contents of a [`Span`]
    fn get_span_contents(&self, span: Span) -> Result<Spanned<Vec<u8>>, ShellError>;
    /// Evaluate a closure passed to the plugin
    fn eval_closure(
        &self,
        closure: Spanned<Closure>,
        positional: Vec<Value>,
        input: PipelineData,
        redirect_stdout: bool,
        redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError>;
    /// Create an owned version of the context with `'static` lifetime
    fn boxed(&self) -> Box<dyn PluginExecutionContext>;
}

/// The execution context of a plugin command. Can be borrowed.
///
/// This is not a public API.
#[doc(hidden)]
pub struct PluginExecutionCommandContext<'a> {
    identity: Arc<PluginIdentity>,
    engine_state: Cow<'a, EngineState>,
    stack: MutableCow<'a, Stack>,
    call: Cow<'a, Call>,
}

impl<'a> PluginExecutionCommandContext<'a> {
    pub fn new(
        identity: Arc<PluginIdentity>,
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        call: &'a Call,
    ) -> PluginExecutionCommandContext<'a> {
        PluginExecutionCommandContext {
            identity,
            engine_state: Cow::Borrowed(engine_state),
            stack: MutableCow::Borrowed(stack),
            call: Cow::Borrowed(call),
        }
    }
}

impl<'a> PluginExecutionContext for PluginExecutionCommandContext<'a> {
    fn span(&self) -> Span {
        self.call.head
    }

    fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        self.engine_state.ctrlc.as_ref()
    }

    fn pipeline_externals_state(&self) -> Option<&Arc<(AtomicU32, AtomicU32)>> {
        Some(&self.engine_state.pipeline_externals_state)
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
                        ClosureEvalOnce::new(&self.engine_state, &self.stack, val)
                            .run_with_input(PipelineData::Empty)
                            .map(|data| data.into_value(span))
                            .unwrap_or_else(|err| Value::error(err, self.call.head))
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

    fn add_env_var(&mut self, name: String, value: Value) -> Result<(), ShellError> {
        self.stack.add_env_var(name, value);
        Ok(())
    }

    fn get_help(&self) -> Result<Spanned<String>, ShellError> {
        let decl = self.engine_state.get_decl(self.call.decl_id);

        Ok(get_full_help(
            &decl.signature(),
            &decl.examples(),
            &self.engine_state,
            &mut self.stack.clone(),
            false,
        )
        .into_spanned(self.call.head))
    }

    fn get_span_contents(&self, span: Span) -> Result<Spanned<Vec<u8>>, ShellError> {
        Ok(self
            .engine_state
            .get_span_contents(span)
            .to_vec()
            .into_spanned(self.call.head))
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

        let mut stack = self
            .stack
            .captures_to_stack(closure.item.captures)
            .reset_pipes();

        let stdout = if redirect_stdout {
            Some(Redirection::Pipe(OutDest::Capture))
        } else {
            None
        };

        let stderr = if redirect_stderr {
            Some(Redirection::Pipe(OutDest::Capture))
        } else {
            None
        };

        let stack = &mut stack.push_redirection(stdout, stderr);

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

        eval_block_with_early_return(&self.engine_state, stack, block, input)
    }

    fn boxed(&self) -> Box<dyn PluginExecutionContext + 'static> {
        Box::new(PluginExecutionCommandContext {
            identity: self.identity.clone(),
            engine_state: Cow::Owned(self.engine_state.clone().into_owned()),
            stack: self.stack.owned(),
            call: Cow::Owned(self.call.clone().into_owned()),
        })
    }
}

/// A bogus execution context for testing that doesn't really implement anything properly
#[cfg(test)]
pub(crate) struct PluginExecutionBogusContext;

#[cfg(test)]
impl PluginExecutionContext for PluginExecutionBogusContext {
    fn span(&self) -> Span {
        Span::test_data()
    }

    fn ctrlc(&self) -> Option<&Arc<AtomicBool>> {
        None
    }

    fn pipeline_externals_state(&self) -> Option<&Arc<(AtomicU32, AtomicU32)>> {
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

    fn add_env_var(&mut self, _name: String, _value: Value) -> Result<(), ShellError> {
        Err(ShellError::NushellFailed {
            msg: "add_env_var not implemented on bogus".into(),
        })
    }

    fn get_help(&self) -> Result<Spanned<String>, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "get_help not implemented on bogus".into(),
        })
    }

    fn get_span_contents(&self, _span: Span) -> Result<Spanned<Vec<u8>>, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "get_span_contents not implemented on bogus".into(),
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

    fn boxed(&self) -> Box<dyn PluginExecutionContext + 'static> {
        Box::new(PluginExecutionBogusContext)
    }
}
