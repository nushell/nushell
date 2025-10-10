use crate::{
    EvalBlockWithEarlyReturnFn, eval_block_with_early_return, get_eval_block_with_early_return,
    redirect_env,
};
use nu_protocol::{
    IntoPipelineData, PipelineData, ShellError, Signature, Span, Value,
    ast::Block,
    debugger::{WithDebug, WithoutDebug},
    engine::{Closure, EngineState, EnvVars, Stack},
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// [`ClosureEval`] is used to repeatedly evaluate a closure with different values/inputs.
///
/// [`ClosureEval`] has a builder API.
/// It is first created via [`ClosureEval::new`],
/// then has arguments added via [`ClosureEval::add_arg`],
/// and then can be run using [`ClosureEval::run_with_input`].
///
/// ```no_run
/// # use nu_protocol::{PipelineData, Value};
/// # use nu_engine::ClosureEval;
/// # let engine_state = unimplemented!();
/// # let stack = unimplemented!();
/// # let closure = unimplemented!();
/// let mut closure = ClosureEval::new(engine_state, stack, closure);
/// let iter = Vec::<Value>::new()
///     .into_iter()
///     .map(move |value| closure.add_arg(value).unwrap().run_with_input(PipelineData::empty()));
/// ```
///
/// Many closures follow a simple, common scheme where the pipeline input and the first argument are the same value.
/// In this case, use [`ClosureEval::run_with_value`]:
///
/// ```no_run
/// # use nu_protocol::{PipelineData, Value};
/// # use nu_engine::ClosureEval;
/// # let engine_state = unimplemented!();
/// # let stack = unimplemented!();
/// # let closure = unimplemented!();
/// let mut closure = ClosureEval::new(engine_state, stack, closure);
/// let iter = Vec::<Value>::new()
///     .into_iter()
///     .map(move |value| closure.run_with_value(value));
/// ```
///
/// Environment isolation and other cleanup is handled by [`ClosureEval`],
/// so nothing needs to be done following [`ClosureEval::run_with_input`] or [`ClosureEval::run_with_value`].
///
/// In contrast to [`ClosureEvalOnce`], [`ClosureEval`] holds an owned copy of the
/// [`EngineState`] supplied. This makes it possible to return it from a function that
/// only has a borrowed [`EngineState`] reference, such as the `run` function of a
/// [`nu_engine::command_prelude::Command`] implementation.
#[derive(Clone)]
pub struct ClosureEval {
    engine_state: EngineState,
    block: Arc<Block>,
    env_vars: Vec<Arc<EnvVars>>,
    env_hidden: Arc<HashMap<String, HashSet<String>>>,
    call_eval: ClosureEvalCommon,
}

impl ClosureEval {
    /// Create a new [`ClosureEval`].
    pub fn new(engine_state: &EngineState, stack: &Stack, closure: Closure) -> Self {
        let engine_state = engine_state.clone();
        let callee_stack = stack.captures_to_stack(closure.captures);
        let block = engine_state.get_block(closure.block_id).clone();
        let env_vars = stack.env_vars.clone();
        let env_hidden = stack.env_hidden.clone();
        let call_eval = ClosureEvalCommon::new(
            callee_stack,
            block.span.unwrap_or(Span::unknown()),
            get_eval_block_with_early_return(&engine_state),
        );

        Self {
            engine_state,
            block,
            env_vars,
            env_hidden,
            call_eval,
        }
    }

    pub fn new_preserve_out_dest(
        engine_state: &EngineState,
        stack: &Stack,
        closure: Closure,
    ) -> Self {
        let engine_state = engine_state.clone();
        let callee_stack = stack.captures_to_stack_preserve_out_dest(closure.captures);
        let block = engine_state.get_block(closure.block_id).clone();
        let env_vars = stack.env_vars.clone();
        let env_hidden = stack.env_hidden.clone();
        let call_eval = ClosureEvalCommon::new(
            callee_stack,
            block.span.unwrap_or(Span::unknown()),
            get_eval_block_with_early_return(&engine_state),
        );

        Self {
            engine_state,
            block,
            env_vars,
            env_hidden,
            call_eval,
        }
    }

    /// Sets whether to enable debugging when evaluating the closure.
    ///
    /// By default, this is controlled by the [`EngineState`] used to create this [`ClosureEval`].
    pub fn debug(&mut self, debug: bool) -> &mut Self {
        self.call_eval.debug(debug);
        self
    }

    /// Add an argument [`Value`] to the closure.
    ///
    /// Multiple [`add_arg`](Self::add_arg) calls can be chained together,
    /// but make sure that arguments are added based on their positional order.
    pub fn add_arg(&mut self, value: Value) -> Result<&mut Self, ShellError> {
        self.call_eval
            .add_positional(&self.block.signature, Cow::Owned(value))?;
        Ok(self)
    }

    /// Run the closure, passing the given [`PipelineData`] as input.
    ///
    /// Any arguments should be added beforehand via [`add_arg`](Self::add_arg).
    pub fn run_with_input(&mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        self.call_eval
            .with_env(&self.env_vars, &self.env_hidden)
            .run(&self.engine_state, &self.block, input)
    }

    /// Run the closure using the given [`Value`] as both the pipeline input and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data())`.
    pub fn run_with_value(&mut self, value: Value) -> Result<PipelineData, ShellError> {
        self.call_eval
            .add_positional(&self.block.signature, Cow::Borrowed(&value))?
            .with_env(&self.env_vars, &self.env_hidden)
            .run(&self.engine_state, &self.block, value.into_pipeline_data())
    }
}

/// [`ClosureEvalOnce`] is used to evaluate a closure a single time.
///
/// [`ClosureEvalOnce`] has a builder API.
/// It is first created via [`ClosureEvalOnce::new`],
/// then has arguments added via [`ClosureEvalOnce::add_arg`],
/// and then can be run using [`ClosureEvalOnce::run_with_input`].
///
/// ```no_run
/// # use nu_protocol::{ListStream, PipelineData, PipelineIterator};
/// # use nu_engine::ClosureEvalOnce;
/// # let engine_state = unimplemented!();
/// # let stack = unimplemented!();
/// # let closure = unimplemented!();
/// # let value = unimplemented!();
/// let result = ClosureEvalOnce::new(engine_state, stack, closure)
///     .add_arg(value)
///     .unwrap()
///     .run_with_input(PipelineData::empty());
/// ```
///
/// Many closures follow a simple, common scheme where the pipeline input and the first argument are the same value.
/// In this case, use [`ClosureEvalOnce::run_with_value`]:
///
/// ```no_run
/// # use nu_protocol::{PipelineData, PipelineIterator};
/// # use nu_engine::ClosureEvalOnce;
/// # let engine_state = unimplemented!();
/// # let stack = unimplemented!();
/// # let closure = unimplemented!();
/// # let value = unimplemented!();
/// let result = ClosureEvalOnce::new(engine_state, stack, closure).run_with_value(value);
/// ```
///
/// In contrast to [`ClosureEval`], the lifetime of [`ClosureEvalOnce`] is bound
/// to that of the supplied [`EngineState`] reference, which makes it more light-weight.
pub struct ClosureEvalOnce<'a> {
    engine_state: &'a EngineState,
    block: &'a Block,
    call_eval: ClosureEvalCommon,
    caller_stack: Option<&'a mut Stack>,
}

impl<'a> ClosureEvalOnce<'a> {
    /// Create a new [`ClosureEvalOnce`].
    pub fn new(engine_state: &'a EngineState, stack: &Stack, closure: Closure) -> Self {
        let block = engine_state.get_block(closure.block_id);
        let callee_stack = stack.captures_to_stack(closure.captures);
        let call_eval = ClosureEvalCommon::new(
            callee_stack,
            block.span.unwrap_or(Span::unknown()),
            get_eval_block_with_early_return(engine_state),
        );
        Self {
            engine_state,
            block,
            call_eval,
            caller_stack: None,
        }
    }

    pub fn new_preserve_out_dest(
        engine_state: &'a EngineState,
        stack: &Stack,
        closure: Closure,
    ) -> Self {
        let block = engine_state.get_block(closure.block_id);
        let callee_stack = stack.captures_to_stack_preserve_out_dest(closure.captures);
        let call_eval = ClosureEvalCommon::new(
            callee_stack,
            block.span.unwrap_or(Span::unknown()),
            get_eval_block_with_early_return(engine_state),
        );
        Self {
            engine_state,
            block,
            call_eval,
            caller_stack: None,
        }
    }

    pub fn new_env_preserve_out_dest(
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        closure: Closure,
    ) -> Self {
        let block = engine_state.get_block(closure.block_id);
        let callee_stack = stack.captures_to_stack_preserve_out_dest(closure.captures);
        let call_eval = ClosureEvalCommon::new(
            callee_stack,
            block.span.unwrap_or(Span::unknown()),
            get_eval_block_with_early_return(engine_state),
        );
        Self {
            engine_state,
            block,
            call_eval,
            caller_stack: Some(stack),
        }
    }

    /// Sets whether to enable debugging when evaluating the closure.
    ///
    /// By default, this is controlled by the [`EngineState`] used to create this [`ClosureEvalOnce`].
    pub fn debug(mut self, debug: bool) -> Self {
        self.call_eval.debug(debug);
        self
    }

    /// Add an argument [`Value`] to the closure.
    ///
    /// Multiple [`add_arg`](Self::add_arg) calls can be chained together,
    /// but make sure that arguments are added based on their positional order.
    pub fn add_arg(mut self, value: Value) -> Result<Self, ShellError> {
        self.call_eval
            .add_positional(&self.block.signature, Cow::Owned(value))?;
        Ok(self)
    }

    /// Add a list of argument [`Value`]s to the closure.
    pub fn add_args(mut self, values: Vec<Value>) -> Result<Self, ShellError> {
        for value in values {
            self.call_eval
                .add_positional(&self.block.signature, Cow::Owned(value))?;
        }
        Ok(self)
    }

    /// Run the closure, passing the given [`PipelineData`] as input.
    ///
    /// Any arguments should be added beforehand via [`add_arg`](Self::add_arg).
    pub fn run_with_input(mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        let result = self.call_eval.run(self.engine_state, self.block, input);
        if let Some(caller) = self.caller_stack {
            self.call_eval.redirect_env(self.engine_state, caller);
        }
        result
    }

    /// Run the closure using the given [`Value`] as both the pipeline input and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data())`.
    pub fn run_with_value(mut self, value: Value) -> Result<PipelineData, ShellError> {
        self.call_eval
            .add_positional(&self.block.signature, Cow::Borrowed(&value))?;
        self.run_with_input(value.into_pipeline_data())
    }
}

/// Code shared between [`ClosureEval`] and [`ClosureEvalOnce`]
#[derive(Clone)]
struct ClosureEvalCommon {
    callee_stack: Stack,
    callee_span: Span,
    arg_index: usize,
    rest_args: Vec<Value>,
    eval: EvalBlockWithEarlyReturnFn,
}

impl ClosureEvalCommon {
    /// Create a new [`CallEval`] context
    pub fn new(callee_stack: Stack, callee_span: Span, eval: EvalBlockWithEarlyReturnFn) -> Self {
        Self {
            callee_stack,
            callee_span,
            arg_index: 0,
            rest_args: Vec::new(),
            eval,
        }
    }

    /// Add a positional argument to the call stack.
    ///
    /// Returns an error if the given `value` does not match the type of
    /// the argument according to the signature (see [`CallEval::new`]).
    pub fn add_positional(
        &mut self,
        signature: &Signature,
        value: Cow<Value>,
    ) -> Result<&mut Self, ShellError> {
        let maybe_param = if self.arg_index < signature.required_positional.len() {
            signature.required_positional.get(self.arg_index)
        } else if self.arg_index
            < (signature.required_positional.len() + signature.optional_positional.len())
        {
            signature
                .optional_positional
                .get(self.arg_index - signature.required_positional.len())
        } else {
            None
        };
        if let Some(param) = maybe_param {
            let param_type = param.shape.to_type();
            if value.is_subtype_of(&param_type) {
                let var_id = param
                    .var_id
                    .expect("internal error: all custom parameters must have var_ids");
                self.callee_stack.add_var(var_id, value.into_owned());
                self.arg_index += 1;
                Ok(self)
            } else {
                Err(ShellError::CantConvert {
                    to_type: param_type.to_string(),
                    from_type: value.get_type().to_string(),
                    span: value.span(),
                    help: None,
                })
            }
        } else {
            // assign arg to rest params
            if let Some(rest_positional) = &signature.rest_positional {
                let param_type = rest_positional.shape.to_type();
                if value.is_subtype_of(&param_type) {
                    self.rest_args.push(value.into_owned());
                    Ok(self)
                } else {
                    Err(ShellError::CantConvert {
                        to_type: param_type.to_string(),
                        from_type: value.get_type().to_string(),
                        span: value.span(),
                        help: None,
                    })
                }
            } else {
                // We do not consider it an error if more arguments
                // are added than the closure takes. This makes it possible
                // to omit any unused arguments in the closure definition.
                Ok(self)
            }
        }
    }

    /// Sets the environment variables for the call.
    pub fn with_env(
        &mut self,
        env_vars: &[Arc<EnvVars>],
        env_hidden: &Arc<HashMap<String, HashSet<String>>>,
    ) -> &mut Self {
        self.callee_stack.with_env(env_vars, env_hidden);
        self
    }

    /// Sets whether to enable debugging when evaluating the closure.
    pub fn debug(&mut self, debug: bool) -> &mut Self {
        if debug {
            self.eval = eval_block_with_early_return::<WithDebug>
        } else {
            self.eval = eval_block_with_early_return::<WithoutDebug>
        };
        self
    }

    /// Run the given block.
    pub fn run(
        &mut self,
        engine_state: &EngineState,
        block: &Block,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        self.finalize_arguments(&block.signature)?;
        self.arg_index = 0;
        self.rest_args.clear();
        (self.eval)(engine_state, &mut self.callee_stack, block, input).map(|p| p.body)
    }

    /// Export the modified environment from callee to the caller.
    pub fn redirect_env(&self, engine_state: &EngineState, stack: &mut Stack) {
        redirect_env(engine_state, stack, &self.callee_stack);
    }

    /// Add default and rest values to the stack, raise error on
    /// missing parameters.
    fn finalize_arguments(&mut self, signature: &Signature) -> Result<(), ShellError> {
        for (num, (param, required)) in signature
            .required_positional
            .iter()
            .map(|p| (p, true))
            .chain(signature.optional_positional.iter().map(|p| (p, false)))
            .enumerate()
        {
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");
            if num < self.arg_index {
                // parameter has been added by add_positional
            } else if let Some(value) = &param.default_value {
                self.callee_stack.add_var(var_id, value.to_owned());
            } else if !required {
                self.callee_stack
                    .add_var(var_id, Value::nothing(self.callee_span.to_owned()));
            } else {
                return Err(ShellError::MissingParameter {
                    param_name: param.name.to_string(),
                    span: self.callee_span.to_owned(),
                });
            }
        }
        if let Some(rest_positional) = &signature.rest_positional {
            let span = if let Some(rest_item) = self.rest_args.first() {
                rest_item.span()
            } else {
                self.callee_span.to_owned()
            };
            self.callee_stack.add_var(
                rest_positional
                    .var_id
                    .expect("Internal error: rest positional parameter lackes var_id"),
                Value::list(self.rest_args.to_owned(), span),
            );
        }
        Ok(())
    }
}

