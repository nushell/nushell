use itertools::Itertools;
use nu_engine::{CallExt, ClosureEval};
use nu_protocol::{
    IntoPipelineData, PipelineData, ShellError, Span, Value,
    engine::{Call, Closure, EngineState, Stack},
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

/// Recursively sort the keys of records in a `Value` tree.
///
/// This ensures that two semantically identical values produce the same
/// serialized representation, even if their record fields appear in different
/// orders. Lists and nested records are traversed recursively.
pub fn sort_attributes(val: Value) -> Value {
    let span = val.span();
    match val {
        Value::Record { val, .. } => {
            let sorted = val
                .into_owned()
                .into_iter()
                .sorted_by(|a, b| a.0.cmp(&b.0))
                .collect_vec();

            let record = sorted
                .into_iter()
                .map(|(k, v)| (k, sort_attributes(v)))
                .collect();

            Value::record(record, span)
        }
        Value::List { vals, .. } => {
            Value::list(vals.into_iter().map(sort_attributes).collect_vec(), span)
        }
        other => other,
    }
}

/// Serialize a `Value` to a NUON string for use as a hash-map key.
///
/// Record keys are sorted before serialization so that two equivalent records
/// with different field ordering produce the same key. This is used by the set
/// operation commands (`union`, `intersect`, `difference`) and by `uniq` for
/// deduplication across all `Value` types.
pub fn value_to_key(
    engine_state: &EngineState,
    value: &Value,
    head: Span,
) -> Result<String, ShellError> {
    let value = sort_attributes(value.clone());
    nuon::to_nuon(
        engine_state,
        &value,
        nuon::ToNuonConfig::default().span(Some(head)),
    )
}

/// Extract and validate the `other` list argument for set operations.
///
/// Used by `union`, `intersect`, and `difference` to parse their required
/// list argument with a consistent error message.
pub fn extract_other_list(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    head: Span,
) -> Result<Vec<Value>, ShellError> {
    let other: Value = call.req(engine_state, stack, 0)?;
    let other_type = other.get_type();
    let other_span = other.span();
    other.into_list().map_err(|_| ShellError::UnsupportedInput {
        msg: "Expected a list from `other` argument".into(),
        input: format!("{}", other_type),
        msg_span: head,
        input_span: other_span,
    })
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
        engine_state.signals().check(&head)?;
        let pred = closure.run_with_value(value)?.into_value(head)?.is_true();

        if pred == accumulator {
            return Ok(Value::bool(accumulator, head).into_pipeline_data());
        }
    }

    Ok(Value::bool(!accumulator, head).into_pipeline_data())
}
