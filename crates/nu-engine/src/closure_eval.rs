use crate::{eval::CallEval, get_eval_block_with_early_return};
use nu_protocol::{
    IntoPipelineData, PipelineData, PipelineMetadata, ShellError, Span, Value,
    ast::Block,
    engine::{Closure, EngineState, EnvName, EnvVars, Stack},
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// [`ClosureEval`] is used to repeatedly evaluate a closure with different values/inputs.
///
/// [`ClosureEval`] has a builder API.
/// It is first created via [`ClosureEval::try_new`],
/// then has arguments added via [`ClosureEval::add_arg`],
/// and then can be run using [`ClosureEval::run_with_input`].
///
/// ```no_run
/// # use nu_protocol::{PipelineData, Value};
/// # use nu_engine::ClosureEval;
/// # let engine_state = unimplemented!();
/// # let stack = unimplemented!();
/// # let closure = unimplemented!();
/// # let span = unimplemented!();
/// let mut closure = ClosureEval::try_new(engine_state, stack, closure, span).unwrap();
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
/// # let span = unimplemented!();
/// let mut closure = ClosureEval::try_new(engine_state, stack, closure, span).unwrap();
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
    env_hidden: Arc<HashMap<String, HashSet<EnvName>>>,
    call_eval: CallEval,
}

impl ClosureEval {
    /// Create a new [`ClosureEval`], or return an error when the closure's
    /// block is not available in this engine state.
    ///
    /// Closures can be passed between jobs. A job may have been spawned before
    /// the closure was parsed, so its cloned engine state may not contain the
    /// referenced block. Keep that case as a regular shell error instead of
    /// panicking while looking up the block. The `span` points at the call or
    /// closure value that triggered the error.
    pub fn try_new(
        engine_state: &EngineState,
        stack: &Stack,
        closure: Closure,
        span: Span,
    ) -> Result<Self, ShellError> {
        Self::try_new_impl(engine_state, stack, closure, span, false)
    }

    /// Create a new [`ClosureEval`] while preserving the caller's output
    /// destination, or return an error when the closure's block is unavailable.
    pub fn try_new_preserve_out_dest(
        engine_state: &EngineState,
        stack: &Stack,
        closure: Closure,
        span: Span,
    ) -> Result<Self, ShellError> {
        Self::try_new_impl(engine_state, stack, closure, span, true)
    }

    fn try_new_impl(
        engine_state: &EngineState,
        stack: &Stack,
        closure: Closure,
        span: Span,
        preserve_out_dest: bool,
    ) -> Result<Self, ShellError> {
        let block = engine_state.get_closure_block(&closure, span)?.clone();
        let engine_state = engine_state.clone();
        let callee_stack = if preserve_out_dest {
            stack.captures_to_stack_preserve_out_dest(closure.captures)
        } else {
            stack.captures_to_stack(closure.captures)
        };
        let env_vars = stack.env_vars.clone();
        let env_hidden = stack.env_hidden.clone();
        let call_eval = CallEval::new(
            callee_stack,
            Span::unknown(),
            block.span.unwrap_or(Span::unknown()),
            get_eval_block_with_early_return(&engine_state),
        );

        Ok(Self {
            engine_state,
            block,
            env_vars,
            env_hidden,
            call_eval,
        })
    }

    /// The block this evaluator runs.
    pub fn block(&self) -> &Block {
        &self.block
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
            .add_positional(&self.block.signature, Cow::Borrowed(&value))?;
        self.run_with_input(value.into_pipeline_data())
    }

    /// Run the closure using the given [`Value`] as both the pipeline input with metadata and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data_with_metadata(metadata))`.
    pub fn run_with_value_with_metadata(
        &mut self,
        value: Value,
        metadata: Option<PipelineMetadata>,
    ) -> Result<PipelineData, ShellError> {
        self.call_eval
            .add_positional(&self.block.signature, Cow::Borrowed(&value))?;
        self.run_with_input(value.into_pipeline_data_with_metadata(metadata))
    }
}

/// [`ClosureEvalOnce`] is used to evaluate a closure a single time.
///
/// [`ClosureEvalOnce`] has a builder API.
/// It is first created via [`ClosureEvalOnce::try_new`],
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
/// # let span = unimplemented!();
/// let result = ClosureEvalOnce::try_new(engine_state, stack, closure, span)
///     .unwrap()
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
/// # let span = unimplemented!();
/// let result = ClosureEvalOnce::try_new(engine_state, stack, closure, span)
///     .unwrap()
///     .run_with_value(value);
/// ```
///
/// In contrast to [`ClosureEval`], the lifetime of [`ClosureEvalOnce`] is bound
/// to that of the supplied [`EngineState`] reference, which makes it more light-weight.
pub struct ClosureEvalOnce<'a> {
    engine_state: &'a EngineState,
    block: &'a Block,
    call_eval: CallEval,
    caller_stack: Option<&'a mut Stack>,
}

impl<'a> ClosureEvalOnce<'a> {
    fn with_block(
        engine_state: &'a EngineState,
        block: &'a Block,
        callee_stack: Stack,
        caller_stack: Option<&'a mut Stack>,
    ) -> Self {
        let call_eval = CallEval::new(
            callee_stack,
            Span::unknown(),
            block.span.unwrap_or(Span::unknown()),
            get_eval_block_with_early_return(engine_state),
        );
        Self {
            engine_state,
            block,
            call_eval,
            caller_stack,
        }
    }

    /// Create a new [`ClosureEvalOnce`] or return an error when the closure's
    /// block is not available in this engine state.
    ///
    /// Closures can be passed between jobs. A job may have been spawned before
    /// the closure was parsed, so its cloned engine state may not contain the
    /// referenced block. Keep that case as a regular shell error instead of
    /// panicking while looking up the block. The `span` points at the call or
    /// closure value that triggered the error.
    pub fn try_new(
        engine_state: &'a EngineState,
        stack: &Stack,
        closure: Closure,
        span: Span,
    ) -> Result<Self, ShellError> {
        let block = engine_state.get_closure_block(&closure, span)?;
        let callee_stack = stack.captures_to_stack(closure.captures);
        Ok(Self::with_block(engine_state, block, callee_stack, None))
    }

    /// Create a new [`ClosureEvalOnce`] while preserving the caller's output
    /// destination, or return an error when the closure's block is unavailable.
    pub fn try_new_preserve_out_dest(
        engine_state: &'a EngineState,
        stack: &Stack,
        closure: Closure,
        span: Span,
    ) -> Result<Self, ShellError> {
        let block = engine_state.get_closure_block(&closure, span)?;
        let callee_stack = stack.captures_to_stack_preserve_out_dest(closure.captures);
        Ok(Self::with_block(engine_state, block, callee_stack, None))
    }

    /// Create a new [`ClosureEvalOnce`] while preserving the caller's
    /// environment and output destination, or return an error when the
    /// closure's block is unavailable.
    pub fn try_new_env_preserve_out_dest(
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        closure: Closure,
        span: Span,
    ) -> Result<Self, ShellError> {
        let block = engine_state.get_closure_block(&closure, span)?;
        let callee_stack = stack.captures_to_stack_preserve_out_dest(closure.captures);
        Ok(Self::with_block(
            engine_state,
            block,
            callee_stack,
            Some(stack),
        ))
    }

    /// The block this evaluator runs.
    pub fn block(&self) -> &Block {
        self.block
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

    /// Run the closure using the given [`Value`] as both the pipeline input with metadata and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data_with_metadata(metadata))`.
    pub fn run_with_value_with_metadata(
        mut self,
        value: Value,
        metadata: Option<PipelineMetadata>,
    ) -> Result<PipelineData, ShellError> {
        self.call_eval
            .add_positional(&self.block.signature, Cow::Borrowed(&value))?;
        self.run_with_input(value.into_pipeline_data_with_metadata(metadata))
    }
}
