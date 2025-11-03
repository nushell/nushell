use crate::{
    EvalBlockWithEarlyReturnFn, eval_block_with_early_return, get_eval_block_with_early_return,
};
use nu_protocol::{
    IntoPipelineData, PipelineData, ShellError, Value,
    ast::Block,
    debugger::{WithDebug, WithoutDebug},
    engine::{Closure, EngineState, EnvVars, Stack},
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

fn eval_fn(debug: bool) -> EvalBlockWithEarlyReturnFn {
    if debug {
        eval_block_with_early_return::<WithDebug>
    } else {
        eval_block_with_early_return::<WithoutDebug>
    }
}

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
///     .map(move |value| closure.add_arg(value).run_with_input(PipelineData::empty()));
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
#[derive(Clone)]
pub struct ClosureEval {
    engine_state: EngineState,
    stack: Stack,
    block: Arc<Block>,
    arg_index: usize,
    env_vars: Vec<Arc<EnvVars>>,
    env_hidden: Arc<HashMap<String, HashSet<String>>>,
    eval: EvalBlockWithEarlyReturnFn,
}

impl ClosureEval {
    /// Create a new [`ClosureEval`].
    pub fn new(engine_state: &EngineState, stack: &Stack, closure: Closure) -> Self {
        let engine_state = engine_state.clone();
        let stack = stack.captures_to_stack(closure.captures);
        let block = engine_state.get_block(closure.block_id).clone();
        let env_vars = stack.env_vars.clone();
        let env_hidden = stack.env_hidden.clone();
        let eval = get_eval_block_with_early_return(&engine_state);

        Self {
            engine_state,
            stack,
            block,
            arg_index: 0,
            env_vars,
            env_hidden,
            eval,
        }
    }

    pub fn new_preserve_out_dest(
        engine_state: &EngineState,
        stack: &Stack,
        closure: Closure,
    ) -> Self {
        let engine_state = engine_state.clone();
        let stack = stack.captures_to_stack_preserve_out_dest(closure.captures);
        let block = engine_state.get_block(closure.block_id).clone();
        let env_vars = stack.env_vars.clone();
        let env_hidden = stack.env_hidden.clone();
        let eval = get_eval_block_with_early_return(&engine_state);

        Self {
            engine_state,
            stack,
            block,
            arg_index: 0,
            env_vars,
            env_hidden,
            eval,
        }
    }

    /// Sets whether to enable debugging when evaluating the closure.
    ///
    /// By default, this is controlled by the [`EngineState`] used to create this [`ClosureEval`].
    pub fn debug(&mut self, debug: bool) -> &mut Self {
        self.eval = eval_fn(debug);
        self
    }

    fn try_add_arg(&mut self, value: Cow<Value>) {
        if let Some(var_id) = self
            .block
            .signature
            .get_positional(self.arg_index)
            .and_then(|var| var.var_id)
        {
            self.stack.add_var(var_id, value.into_owned());
            self.arg_index += 1;
        }
    }

    /// Add an argument [`Value`] to the closure.
    ///
    /// Multiple [`add_arg`](Self::add_arg) calls can be chained together,
    /// but make sure that arguments are added based on their positional order.
    pub fn add_arg(&mut self, value: Value) -> &mut Self {
        self.try_add_arg(Cow::Owned(value));
        self
    }

    /// Run the closure, passing the given [`PipelineData`] as input.
    ///
    /// Any arguments should be added beforehand via [`add_arg`](Self::add_arg).
    pub fn run_with_input(&mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        self.arg_index = 0;
        self.stack.with_env(&self.env_vars, &self.env_hidden);
        (self.eval)(&self.engine_state, &mut self.stack, &self.block, input).map(|p| p.body)
    }

    /// Run the closure using the given [`Value`] as both the pipeline input and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data())`.
    pub fn run_with_value(&mut self, value: Value) -> Result<PipelineData, ShellError> {
        self.try_add_arg(Cow::Borrowed(&value));
        self.run_with_input(value.into_pipeline_data())
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
pub struct ClosureEvalOnce<'a> {
    engine_state: &'a EngineState,
    stack: Stack,
    block: &'a Block,
    arg_index: usize,
    eval: EvalBlockWithEarlyReturnFn,
}

impl<'a> ClosureEvalOnce<'a> {
    /// Create a new [`ClosureEvalOnce`].
    pub fn new(engine_state: &'a EngineState, stack: &Stack, closure: Closure) -> Self {
        let block = engine_state.get_block(closure.block_id);
        let eval = get_eval_block_with_early_return(engine_state);
        Self {
            engine_state,
            stack: stack.captures_to_stack(closure.captures),
            block,
            arg_index: 0,
            eval,
        }
    }

    pub fn new_preserve_out_dest(
        engine_state: &'a EngineState,
        stack: &Stack,
        closure: Closure,
    ) -> Self {
        let block = engine_state.get_block(closure.block_id);
        let eval = get_eval_block_with_early_return(engine_state);
        Self {
            engine_state,
            stack: stack.captures_to_stack_preserve_out_dest(closure.captures),
            block,
            arg_index: 0,
            eval,
        }
    }

    /// Sets whether to enable debugging when evaluating the closure.
    ///
    /// By default, this is controlled by the [`EngineState`] used to create this [`ClosureEvalOnce`].
    pub fn debug(mut self, debug: bool) -> Self {
        self.eval = eval_fn(debug);
        self
    }

    fn try_add_arg(&mut self, value: Cow<Value>) {
        if let Some(var_id) = self
            .block
            .signature
            .get_positional(self.arg_index)
            .and_then(|var| var.var_id)
        {
            self.stack.add_var(var_id, value.into_owned());
            self.arg_index += 1;
        }
    }

    /// Add an argument [`Value`] to the closure.
    ///
    /// Multiple [`add_arg`](Self::add_arg) calls can be chained together,
    /// but make sure that arguments are added based on their positional order.
    pub fn add_arg(mut self, value: Value) -> Self {
        self.try_add_arg(Cow::Owned(value));
        self
    }

    /// Run the closure, passing the given [`PipelineData`] as input.
    ///
    /// Any arguments should be added beforehand via [`add_arg`](Self::add_arg).
    pub fn run_with_input(mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        (self.eval)(self.engine_state, &mut self.stack, self.block, input).map(|p| p.body)
    }

    /// Run the closure using the given [`Value`] as both the pipeline input and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data())`.
    pub fn run_with_value(mut self, value: Value) -> Result<PipelineData, ShellError> {
        self.try_add_arg(Cow::Borrowed(&value));
        self.run_with_input(value.into_pipeline_data())
    }
}
