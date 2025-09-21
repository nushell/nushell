use crate::util::MutableCow;
use nu_engine::{ClosureEvalOnce, get_eval_block_with_early_return, get_full_help};
use nu_plugin_protocol::EvaluatedCall;
use nu_protocol::{
    Config, DeclId, IntoSpanned, OutDest, PipelineData, PluginIdentity, ShellError, Signals, Span,
    Spanned, Value,
    engine::{Call, Closure, EngineState, Redirection, Stack},
    ir,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Arc, atomic::AtomicU32},
};

/// Object safe trait for abstracting operations required of the plugin context.
pub trait PluginExecutionContext: Send + Sync {
    /// A span pointing to the command being executed
    fn span(&self) -> Span;
    /// The [`Signals`] struct, if present
    fn signals(&self) -> &Signals;
    /// The pipeline externals state, for tracking the foreground process group, if present
    fn pipeline_externals_state(&self) -> Option<&Arc<(AtomicU32, AtomicU32)>>;
    /// Get engine configuration
    fn get_config(&self) -> Result<Arc<Config>, ShellError>;
    /// Get plugin configuration
    fn get_plugin_config(&self) -> Result<Option<Value>, ShellError>;
    /// Get an environment variable from `$env`
    fn get_env_var(&self, name: &str) -> Result<Option<&Value>, ShellError>;
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
    /// Find a declaration by name
    fn find_decl(&self, name: &str) -> Result<Option<DeclId>, ShellError>;
    /// Call a declaration with arguments and input
    fn call_decl(
        &mut self,
        decl_id: DeclId,
        call: EvaluatedCall,
        input: PipelineData,
        redirect_stdout: bool,
        redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError>;
    /// Create an owned version of the context with `'static` lifetime
    fn boxed(&self) -> Box<dyn PluginExecutionContext>;
}

/// The execution context of a plugin command. Can be borrowed.
pub struct PluginExecutionCommandContext<'a> {
    identity: Arc<PluginIdentity>,
    engine_state: Cow<'a, EngineState>,
    stack: MutableCow<'a, Stack>,
    call: Call<'a>,
}

impl<'a> PluginExecutionCommandContext<'a> {
    pub fn new(
        identity: Arc<PluginIdentity>,
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        call: &'a Call<'a>,
    ) -> PluginExecutionCommandContext<'a> {
        PluginExecutionCommandContext {
            identity,
            engine_state: Cow::Borrowed(engine_state),
            stack: MutableCow::Borrowed(stack),
            call: call.clone(),
        }
    }
}

impl PluginExecutionContext for PluginExecutionCommandContext<'_> {
    fn span(&self) -> Span {
        self.call.head
    }

    fn signals(&self) -> &Signals {
        self.engine_state.signals()
    }

    fn pipeline_externals_state(&self) -> Option<&Arc<(AtomicU32, AtomicU32)>> {
        Some(&self.engine_state.pipeline_externals_state)
    }

    fn get_config(&self) -> Result<Arc<Config>, ShellError> {
        Ok(self.stack.get_config(&self.engine_state))
    }

    fn get_plugin_config(&self) -> Result<Option<Value>, ShellError> {
        // Fetch the configuration for a plugin
        //
        // The `plugin` must match the registered name of a plugin.  For `plugin add
        // nu_plugin_example` the plugin config lookup uses `"example"`
        Ok(self
            .get_config()?
            .plugins
            .get(self.identity.name())
            .cloned()
            .map(|value| {
                let span = value.span();
                match value {
                    Value::Closure { val, .. } => {
                        ClosureEvalOnce::new(&self.engine_state, &self.stack, *val)
                            .run_with_input(PipelineData::empty())
                            .and_then(|data| data.into_value(span))
                            .unwrap_or_else(|err| Value::error(err, self.call.head))
                    }
                    _ => value.clone(),
                }
            }))
    }

    fn get_env_var(&self, name: &str) -> Result<Option<&Value>, ShellError> {
        Ok(self
            .stack
            .get_env_var_insensitive(&self.engine_state, name)
            .map(|(_, value)| value))
    }

    fn get_env_vars(&self) -> Result<HashMap<String, Value>, ShellError> {
        Ok(self.stack.get_env_vars(&self.engine_state))
    }

    fn get_current_dir(&self) -> Result<Spanned<String>, ShellError> {
        #[allow(deprecated)]
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

        Ok(
            get_full_help(decl, &self.engine_state, &mut self.stack.clone())
                .into_spanned(self.call.head),
        )
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
                    closure.item.block_id.get()
                ),
                span: Some(closure.span),
                help: None,
                inner: vec![],
            })?;

        let mut stack = self
            .stack
            .captures_to_stack(closure.item.captures)
            .reset_pipes();

        let stack = &mut stack.push_redirection(
            redirect_stdout.then_some(Redirection::Pipe(OutDest::PipeSeparate)),
            redirect_stderr.then_some(Redirection::Pipe(OutDest::PipeSeparate)),
        );

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

        eval_block_with_early_return(&self.engine_state, stack, block, input).map(|p| p.body)
    }

    fn find_decl(&self, name: &str) -> Result<Option<DeclId>, ShellError> {
        Ok(self.engine_state.find_decl(name.as_bytes(), &[]))
    }

    fn call_decl(
        &mut self,
        decl_id: DeclId,
        call: EvaluatedCall,
        input: PipelineData,
        redirect_stdout: bool,
        redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError> {
        if decl_id.get() >= self.engine_state.num_decls() {
            return Err(ShellError::GenericError {
                error: "Plugin misbehaving".into(),
                msg: format!("Tried to call unknown decl id: {}", decl_id.get()),
                span: Some(call.head),
                help: None,
                inner: vec![],
            });
        }

        let decl = self.engine_state.get_decl(decl_id);

        let stack = &mut self.stack.push_redirection(
            redirect_stdout.then_some(Redirection::Pipe(OutDest::PipeSeparate)),
            redirect_stderr.then_some(Redirection::Pipe(OutDest::PipeSeparate)),
        );

        let mut call_builder = ir::Call::build(decl_id, call.head);

        for positional in call.positional {
            call_builder.add_positional(stack, positional.span(), positional);
        }

        for (name, value) in call.named {
            if let Some(value) = value {
                call_builder.add_named(stack, &name.item, "", name.span, value);
            } else {
                call_builder.add_flag(stack, &name.item, "", name.span);
            }
        }

        call_builder.with(stack, |stack, call| {
            decl.run(&self.engine_state, stack, call, input)
        })
    }

    fn boxed(&self) -> Box<dyn PluginExecutionContext + 'static> {
        Box::new(PluginExecutionCommandContext {
            identity: self.identity.clone(),
            engine_state: Cow::Owned(self.engine_state.clone().into_owned()),
            stack: self.stack.owned(),
            call: self.call.to_owned(),
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

    fn signals(&self) -> &Signals {
        &Signals::EMPTY
    }

    fn pipeline_externals_state(&self) -> Option<&Arc<(AtomicU32, AtomicU32)>> {
        None
    }

    fn get_config(&self) -> Result<Arc<Config>, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "get_config not implemented on bogus".into(),
        })
    }

    fn get_plugin_config(&self) -> Result<Option<Value>, ShellError> {
        Ok(None)
    }

    fn get_env_var(&self, _name: &str) -> Result<Option<&Value>, ShellError> {
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

    fn find_decl(&self, _name: &str) -> Result<Option<DeclId>, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "find_decl not implemented on bogus".into(),
        })
    }

    fn call_decl(
        &mut self,
        _decl_id: DeclId,
        _call: EvaluatedCall,
        _input: PipelineData,
        _redirect_stdout: bool,
        _redirect_stderr: bool,
    ) -> Result<PipelineData, ShellError> {
        Err(ShellError::NushellFailed {
            msg: "call_decl not implemented on bogus".into(),
        })
    }

    fn boxed(&self) -> Box<dyn PluginExecutionContext + 'static> {
        Box::new(PluginExecutionBogusContext)
    }
}
