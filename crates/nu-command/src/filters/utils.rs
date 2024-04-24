use nu_engine::{CallExt, ClosureEval};
use nu_protocol::{
    ast::Call,
    engine::{Closure, EngineState, Stack},
    IntoPipelineData, PipelineData, ShellError, Span, Value,
};

pub fn chain_error_with_input(
    error_source: ShellError,
    input_is_error: bool,
    span: Span,
) -> ShellError {
    if !input_is_error {
        return ShellError::EvalBlockWithInput {
            span,
            sources: vec![error_source],
        };
    }
    error_source
}

pub fn boolean_fold(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    accumulator: bool,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let closure: Closure = call.req(engine_state, stack, 0)?;

    let mut closure = ClosureEval::new(engine_state, stack, closure);

    for value in input {
        if nu_utils::ctrl_c::was_pressed(&engine_state.ctrlc) {
            break;
        }

        let pred = closure.run_with_value(value)?.into_value(head).is_true();

        if pred == accumulator {
            return Ok(Value::bool(accumulator, head).into_pipeline_data());
        }
    }

    Ok(Value::bool(!accumulator, head).into_pipeline_data())
}
