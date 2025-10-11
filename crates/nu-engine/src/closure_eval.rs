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
pub struct ClosureEval {
    engine_state: EngineState,
    stack: Stack,
    block: Arc<Block>,
    arg_index: usize,
    rest_positional: Vec<Value>,
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
            rest_positional: Vec::new(),
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
            rest_positional: Vec::new(),
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

    /// Add an argument [`Value`] to the closure.
    ///
    /// Multiple [`add_arg`](Self::add_arg) calls can be chained together,
    /// but make sure that arguments are added based on their positional order.
    pub fn add_arg(&mut self, value: Value) -> Result<&mut Self, ShellError> {
        try_add_arg(
            &mut self.stack,
            &self.block.signature,
            &mut self.arg_index,
            Cow::Owned(value),
            &mut self.rest_positional,
        )?;
        Ok(self)
    }

    /// Run the closure, passing the given [`PipelineData`] as input.
    ///
    /// Any arguments should be added beforehand via [`add_arg`](Self::add_arg).
    pub fn run_with_input(&mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        finalize_arguments(
            &mut self.stack,
            &self.block.signature,
            &mut self.arg_index,
            &mut self.rest_positional,
            &self.block,
        )?;
        self.arg_index = 0;
        self.stack.with_env(&self.env_vars, &self.env_hidden);
        (self.eval)(&self.engine_state, &mut self.stack, &self.block, input).map(|p| p.body)
    }

    /// Run the closure using the given [`Value`] as both the pipeline input and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data())`.
    pub fn run_with_value(&mut self, value: Value) -> Result<PipelineData, ShellError> {
        try_add_arg(
            &mut self.stack,
            &self.block.signature,
            &mut self.arg_index,
            Cow::Borrowed(&value),
            &mut self.rest_positional,
        )?;
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
    stack: Stack,
    caller_stack: Option<&'a mut Stack>,
    block: &'a Block,
    arg_index: usize,
    rest_positional: Vec<Value>,
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
            caller_stack: None,
            block,
            arg_index: 0,
            rest_positional: Vec::new(),
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
            caller_stack: None,
            block,
            arg_index: 0,
            rest_positional: Vec::new(),
            eval,
        }
    }

    pub fn new_env_preserve_out_dest(
        engine_state: &'a EngineState,
        stack: &'a mut Stack,
        closure: Closure,
    ) -> Self {
        let block = engine_state.get_block(closure.block_id);
        let eval = get_eval_block_with_early_return(engine_state);
        Self {
            engine_state,
            stack: stack.captures_to_stack_preserve_out_dest(closure.captures),
            caller_stack: Some(stack),
            block,
            arg_index: 0,
            rest_positional: Vec::new(),
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

    /// Add an argument [`Value`] to the closure.
    ///
    /// Multiple [`add_arg`](Self::add_arg) calls can be chained together,
    /// but make sure that arguments are added based on their positional order.
    pub fn add_arg(mut self, value: Value) -> Result<Self, ShellError> {
        try_add_arg(
            &mut self.stack,
            &self.block.signature,
            &mut self.arg_index,
            Cow::Owned(value),
            &mut self.rest_positional,
        )?;
        Ok(self)
    }

    /// Add a list of argument [`Value`]s to the closure.
    pub fn add_args(mut self, values: Vec<Value>) -> Result<Self, ShellError> {
        for value in values {
            try_add_arg(
                &mut self.stack,
                &self.block.signature,
                &mut self.arg_index,
                Cow::Owned(value),
                &mut self.rest_positional,
            )?;
        }
        Ok(self)
    }

    /// Run the closure, passing the given [`PipelineData`] as input.
    ///
    /// Any arguments should be added beforehand via [`add_arg`](Self::add_arg).
    pub fn run_with_input(mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        finalize_arguments(
            &mut self.stack,
            &self.block.signature,
            &mut self.arg_index,
            &mut self.rest_positional,
            self.block,
        )?;
        let result =
            (self.eval)(self.engine_state, &mut self.stack, self.block, input).map(|p| p.body);
        if let Some(caller) = self.caller_stack {
            redirect_env(self.engine_state, caller, &self.stack);
        }
        result
    }

    /// Run the closure using the given [`Value`] as both the pipeline input and the first argument.
    ///
    /// Using this function after or in combination with [`add_arg`](Self::add_arg) is most likely an error.
    /// This function is equivalent to `self.add_arg(value)` followed by `self.run_with_input(value.into_pipeline_data())`.
    pub fn run_with_value(mut self, value: Value) -> Result<PipelineData, ShellError> {
        try_add_arg(
            &mut self.stack,
            &self.block.signature,
            &mut self.arg_index,
            Cow::Borrowed(&value),
            &mut self.rest_positional,
        )?;
        self.run_with_input(value.into_pipeline_data())
    }
}

fn try_add_arg(
    stack: &mut Stack,
    signature: &Signature,
    arg_index: &mut usize,
    value: Cow<Value>,
    rest_positional: &mut Vec<Value>,
) -> Result<(), ShellError> {
    let maybe_param = if *arg_index < signature.required_positional.len() {
        signature.required_positional.get(*arg_index)
    } else if *arg_index
        < (signature.required_positional.len() + signature.optional_positional.len())
    {
        signature
            .optional_positional
            .get(*arg_index - signature.required_positional.len())
    } else {
        None
    };
    if let Some(param) = maybe_param {
        let param_type = param.shape.to_type();
        if value.is_subtype_of(&param_type) {
            let var_id = param
                .var_id
                .expect("internal error: all custom parameters must have var_ids");
            stack.add_var(var_id, value.into_owned());
            *arg_index += 1;
            Ok(())
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
        rest_positional.push(value.into_owned());
        Ok(())
    }
}

/// Add default and rest values to the stack, raise error on
/// missing parameters.
fn finalize_arguments(
    stack: &mut Stack,
    signature: &Signature,
    arg_index: &mut usize,
    rest_args: &mut [Value],
    block: &Block,
) -> Result<(), ShellError> {
    let closure_span = block.span.unwrap_or(Span::unknown());
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
        if num < *arg_index {
            // parameter has been added by try_add_arg
        } else if let Some(value) = &param.default_value {
            stack.add_var(var_id, value.to_owned());
        } else if !required {
            stack.add_var(var_id, Value::nothing(closure_span.to_owned()));
        } else {
            return Err(ShellError::MissingParameter {
                param_name: param.name.to_string(),
                span: closure_span.to_owned(),
            });
        }
    }
    if let Some(rest_positional) = &signature.rest_positional {
        let span = if let Some(rest_item) = rest_args.first() {
            rest_item.span()
        } else {
            closure_span.to_owned()
        };
        stack.add_var(
            rest_positional
                .var_id
                .expect("Internal error: rest positional parameter lackes var_id"),
            Value::list(rest_args.to_owned(), span),
        );
    }
    Ok(())
}
