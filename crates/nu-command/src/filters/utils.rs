use nu_engine::{eval_block, CallExt};
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
    let span = call.head;

    let capture_block: Closure = call.req(engine_state, stack, 0)?;
    let block_id = capture_block.block_id;

    let block = engine_state.get_block(block_id);
    let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);
    let mut stack = stack.captures_to_stack(capture_block.captures);

    let orig_env_vars = stack.env_vars.clone();
    let orig_env_hidden = stack.env_hidden.clone();

    let ctrlc = engine_state.ctrlc.clone();

    for value in input.into_interruptible_iter(ctrlc) {
        // with_env() is used here to ensure that each iteration uses
        // a different set of environment variables.
        // Hence, a 'cd' in the first loop won't affect the next loop.
        stack.with_env(&orig_env_vars, &orig_env_hidden);

        if let Some(var_id) = var_id {
            stack.add_var(var_id, value.clone());
        }

        let eval = eval_block(
            engine_state,
            &mut stack,
            block,
            value.into_pipeline_data(),
            call.redirect_stdout,
            call.redirect_stderr,
        );
        match eval {
            Err(e) => {
                return Err(e);
            }
            Ok(pipeline_data) => {
                if pipeline_data.into_value(span).is_true() == accumulator {
                    return Ok(Value::bool(accumulator, span).into_pipeline_data());
                }
            }
        }
    }

    Ok(Value::bool(!accumulator, span).into_pipeline_data())
}
