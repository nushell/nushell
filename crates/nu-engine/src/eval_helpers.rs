use crate::{
    eval_block, eval_block_with_early_return, eval_expression, eval_expression_with_input,
    eval_subexpression,
};
use nu_protocol::ast::{Block, Expression};
use nu_protocol::debugger::{WithDebug, WithoutDebug};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Span, Value};

/// Type of eval_block() function
pub type EvalBlockFn = fn(
    &EngineState,
    &mut Stack,
    &Block,
    PipelineData,
    bool,
    bool,
) -> Result<PipelineData, ShellError>;

/// Type of eval_block_with_early_return() function
pub type EvalBlockWithEarlyReturnFn = fn(
    &EngineState,
    &mut Stack,
    &Block,
    PipelineData,
    bool,
    bool,
) -> Result<PipelineData, ShellError>;

/// Type of eval_expression() function
pub type EvalExpressionFn = fn(&EngineState, &mut Stack, &Expression) -> Result<Value, ShellError>;

/// Type of eval_expression_with_input() function
pub type EvalExpressionWithInputFn = fn(
    &EngineState,
    &mut Stack,
    &Expression,
    PipelineData,
    bool,
    bool,
) -> Result<(PipelineData, bool), ShellError>;

/// Type of eval_subexpression() function
pub type EvalSubexpressionFn =
    fn(&EngineState, &mut Stack, &Block, PipelineData) -> Result<PipelineData, ShellError>;

/// Helper function to fetch `eval_block_with_early_return()` with the correct type parameter based
/// on whether engine_state is configured with or without a debugger.
pub fn get_eval_block_with_early_return(
    engine_state: &EngineState,
    span: Span,
) -> Result<EvalBlockWithEarlyReturnFn, ShellError> {
    if let Ok(debugger) = engine_state.debugger.lock() {
        Ok(if debugger.should_debug() {
            eval_block_with_early_return::<WithDebug>
        } else {
            eval_block_with_early_return::<WithoutDebug>
        })
    } else {
        Err(ShellError::GenericError {
            error: "Internal Error: Could not lock debugger".to_string(),
            msg: "Could not lock debugger".to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }
}

/// Helper function to fetch `eval_block()` with the correct type parameter based on whether
/// engine_state is configured with or without a debugger.
pub fn get_eval_block(engine_state: &EngineState, span: Span) -> Result<EvalBlockFn, ShellError> {
    if let Ok(debugger) = engine_state.debugger.lock() {
        Ok(if debugger.should_debug() {
            eval_block::<WithDebug>
        } else {
            eval_block::<WithoutDebug>
        })
    } else {
        Err(ShellError::GenericError {
            error: "Internal Error: Could not lock debugger".to_string(),
            msg: "Could not lock debugger".to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }
}

/// Helper function to fetch `eval_expression()` with the correct type parameter based on whether
/// engine_state is configured with or without a debugger.
pub fn get_eval_expression(
    engine_state: &EngineState,
    span: Span,
) -> Result<EvalExpressionFn, ShellError> {
    if let Ok(debugger) = engine_state.debugger.lock() {
        Ok(if debugger.should_debug() {
            eval_expression::<WithDebug>
        } else {
            eval_expression::<WithoutDebug>
        })
    } else {
        Err(ShellError::GenericError {
            error: "Internal Error: Could not lock debugger".to_string(),
            msg: "Could not lock debugger".to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }
}

/// Helper function to fetch `eval_expression_with_input()` with the correct type parameter based
/// on whether engine_state is configured with or without a debugger.
pub fn get_eval_expression_with_input(
    engine_state: &EngineState,
    span: Span,
) -> Result<EvalExpressionWithInputFn, ShellError> {
    if let Ok(debugger) = engine_state.debugger.lock() {
        Ok(if debugger.should_debug() {
            eval_expression_with_input::<WithDebug>
        } else {
            eval_expression_with_input::<WithoutDebug>
        })
    } else {
        Err(ShellError::GenericError {
            error: "Internal Error: Could not lock debugger".to_string(),
            msg: "Could not lock debugger".to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }
}

/// Helper function to fetch `eval_subexpression()` with the correct type parameter based on whether
/// engine_state is configured with or without a debugger.
pub fn get_eval_subexpression(
    engine_state: &EngineState,
    span: Span,
) -> Result<EvalSubexpressionFn, ShellError> {
    if let Ok(debugger) = engine_state.debugger.lock() {
        Ok(if debugger.should_debug() {
            eval_subexpression::<WithDebug>
        } else {
            eval_subexpression::<WithoutDebug>
        })
    } else {
        Err(ShellError::GenericError {
            error: "Internal Error: Could not lock debugger".to_string(),
            msg: "Could not lock debugger".to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
    }
}
