use crate::{get_eval_block_with_early_return, EvalBlockWithEarlyReturnFn};
use nu_protocol::{
    ast::Block,
    engine::{Closure, EngineState, EnvVars, Stack},
    IntoPipelineData, PipelineData, ShellError, Value,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub struct ClosureEval {
    engine_state: EngineState,
    stack: Stack,
    block: Arc<Block>,
    arg_index: usize,
    env_vars: Vec<EnvVars>,
    env_hidden: HashMap<String, HashSet<String>>,
    eval: EvalBlockWithEarlyReturnFn,
}

impl ClosureEval {
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

    pub fn add_arg(&mut self, value: Value) -> &mut Self {
        self.try_add_arg(Cow::Owned(value));
        self
    }

    pub fn run_with_input(&mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        self.arg_index = 0;
        self.stack.with_env(&self.env_vars, &self.env_hidden);
        (self.eval)(&self.engine_state, &mut self.stack, &self.block, input)
    }

    pub fn run_with_value(&mut self, value: Value) -> Result<PipelineData, ShellError> {
        self.try_add_arg(Cow::Borrowed(&value));
        self.run_with_input(value.into_pipeline_data())
    }
}

pub struct ClosureEvalOnce<'a> {
    engine_state: &'a EngineState,
    stack: Stack,
    block: &'a Block,
    arg_index: usize,
}

impl<'a> ClosureEvalOnce<'a> {
    pub fn new(engine_state: &'a EngineState, stack: &Stack, closure: Closure) -> Self {
        let block = engine_state.get_block(closure.block_id);
        Self {
            engine_state,
            stack: stack.captures_to_stack(closure.captures),
            block,
            arg_index: 0,
        }
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

    pub fn add_arg(mut self, value: Value) -> Self {
        self.try_add_arg(Cow::Owned(value));
        self
    }

    pub fn run_with_input(mut self, input: PipelineData) -> Result<PipelineData, ShellError> {
        let eval = get_eval_block_with_early_return(self.engine_state);
        eval(self.engine_state, &mut self.stack, self.block, input)
    }

    pub fn run_with_value(mut self, value: Value) -> Result<PipelineData, ShellError> {
        self.try_add_arg(Cow::Borrowed(&value));
        self.run_with_input(value.into_pipeline_data())
    }
}
