use crate::{
    eval_block, eval_block_with_early_return, eval_expression, eval_expression_with_input,
    eval_ir_block, eval_subexpression,
};
use nu_protocol::{
    PipelineData, PipelineExecutionData, ShellError, Value,
    ast::{Block, Expression},
    debugger::{WithDebug, WithoutDebug},
    engine::{EngineState, Stack},
};

/// Type of eval_block() function
pub type EvalBlockFn =
    fn(&EngineState, &mut Stack, &Block, PipelineData) -> Result<PipelineExecutionData, ShellError>;

/// Type of eval_ir_block() function
pub type EvalIrBlockFn =
    fn(&EngineState, &mut Stack, &Block, PipelineData) -> Result<PipelineExecutionData, ShellError>;

/// Type of eval_block_with_early_return() function
pub type EvalBlockWithEarlyReturnFn =
    fn(&EngineState, &mut Stack, &Block, PipelineData) -> Result<PipelineExecutionData, ShellError>;

/// Type of eval_expression() function
pub type EvalExpressionFn = fn(&EngineState, &mut Stack, &Expression) -> Result<Value, ShellError>;

/// Type of eval_expression_with_input() function
pub type EvalExpressionWithInputFn =
    fn(&EngineState, &mut Stack, &Expression, PipelineData) -> Result<PipelineData, ShellError>;

/// Type of eval_subexpression() function
pub type EvalSubexpressionFn =
    fn(&EngineState, &mut Stack, &Block, PipelineData) -> Result<PipelineData, ShellError>;

/// Helper function to fetch `eval_block()` with the correct type parameter based on whether
/// engine_state is configured with or without a debugger.
pub fn get_eval_block(engine_state: &EngineState) -> EvalBlockFn {
    if engine_state.is_debugging() {
        eval_block::<WithDebug>
    } else {
        eval_block::<WithoutDebug>
    }
}

/// Helper function to fetch `eval_ir_block()` with the correct type parameter based on whether
/// engine_state is configured with or without a debugger.
pub fn get_eval_ir_block(engine_state: &EngineState) -> EvalIrBlockFn {
    if engine_state.is_debugging() {
        eval_ir_block::<WithDebug>
    } else {
        eval_ir_block::<WithoutDebug>
    }
}

/// Helper function to fetch `eval_block_with_early_return()` with the correct type parameter based
/// on whether engine_state is configured with or without a debugger.
pub fn get_eval_block_with_early_return(engine_state: &EngineState) -> EvalBlockWithEarlyReturnFn {
    if engine_state.is_debugging() {
        eval_block_with_early_return::<WithDebug>
    } else {
        eval_block_with_early_return::<WithoutDebug>
    }
}

/// Helper function to fetch `eval_expression()` with the correct type parameter based on whether
/// engine_state is configured with or without a debugger.
pub fn get_eval_expression(engine_state: &EngineState) -> EvalExpressionFn {
    if engine_state.is_debugging() {
        eval_expression::<WithDebug>
    } else {
        eval_expression::<WithoutDebug>
    }
}

/// Helper function to fetch `eval_expression_with_input()` with the correct type parameter based
/// on whether engine_state is configured with or without a debugger.
pub fn get_eval_expression_with_input(engine_state: &EngineState) -> EvalExpressionWithInputFn {
    if engine_state.is_debugging() {
        eval_expression_with_input::<WithDebug>
    } else {
        eval_expression_with_input::<WithoutDebug>
    }
}

/// Helper function to fetch `eval_subexpression()` with the correct type parameter based on whether
/// engine_state is configured with or without a debugger.
pub fn get_eval_subexpression(engine_state: &EngineState) -> EvalSubexpressionFn {
    if engine_state.is_debugging() {
        eval_subexpression::<WithDebug>
    } else {
        eval_subexpression::<WithoutDebug>
    }
}
