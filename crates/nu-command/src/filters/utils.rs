use std::ops::Deref;

use nu_engine::{CallExt, ClosureEval};
use nu_protocol::{
    IntoPipelineData, PipelineData, ShellError, ShellWarning, Span, Value,
    engine::{Call, EngineState, Stack},
    report_shell_warning,
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

// FIXME: Revert changes to this function after deprecation period
pub fn boolean_fold(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    accumulator: bool,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let predicate: Value = call.req(engine_state, stack, 0)?;
    match predicate {
        Value::Closure { val, .. } => {
            let mut closure = ClosureEval::new(engine_state, stack, val.deref().clone());
            for value in input {
                engine_state.signals().check(&head)?;
                let pred = closure.run_with_value(value)?.into_value(head)?.is_true();

                if pred == accumulator {
                    return Ok(Value::bool(accumulator, head).into_pipeline_data());
                }
            }
            Ok(Value::bool(!accumulator, head).into_pipeline_data())
        }
        Value::Record {
            val,
            internal_span: span,
            ..
        } if val.is_empty() => {
            report_shell_warning(
                Some(stack),
                engine_state,
                &ShellWarning::Deprecated {
                    dep_type: "Argument".to_string(),
                    label: "Replace `{}` with `$it` or `{||}`".to_string(),
                    span,
                    help: Some("Changes to this command's signature mean that `{}` is no longer parsed as an empty closure.
To achieve the same outcome, use one of the two other forms.".to_string()),
                    report_mode: nu_protocol::ReportMode::FirstUse,
                },
            );
            for value in input {
                engine_state.signals().check(&head)?;
                if value.is_true() == accumulator {
                    return Ok(Value::bool(accumulator, head).into_pipeline_data());
                }
            }
            Ok(Value::bool(!accumulator, head).into_pipeline_data())
        }
        val => Err(ShellError::RuntimeTypeMismatch {
            expected: nu_protocol::Type::Closure,
            actual: val.get_type(),
            span: val.span(),
        }),
    }
}
