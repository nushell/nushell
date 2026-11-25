use crate::{eval_block, eval_block_with_early_return, eval_ir_block};
use nu_protocol::{
    PipelineData, PipelineExecutionData, ShellError,
    ast::Block,
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
