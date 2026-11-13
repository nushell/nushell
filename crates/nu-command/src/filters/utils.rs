use itertools::Itertools;
use nu_engine::{CallExt, ClosureEval};
use nu_protocol::{
    IntoPipelineData, PipelineData, ShellError, Span, Value,
    engine::{Call, Closure, EngineState, Stack},
    shell_error::eval_block_with_input::EvalBlockWithInputError,
};

/// Wraps an error a closure raised while running on one input value, so the error reports as
/// itself with a `source value` label at `span`, the input value's span.
///
/// An input that is already an error is not labeled: the closure most likely re-raised it.
/// See [`EvalBlockWithInputError::wrap`] for the errors that are never wrapped.
pub fn wrap_error_with_input(error: ShellError, input_is_error: bool, span: Span) -> ShellError {
    if input_is_error {
        return error;
    }
    EvalBlockWithInputError::wrap(error, span, EvalBlockWithInputError::SOURCE_VALUE)
}

/// Like [`Iterator::skip`], but an item that holds an error is kept, since skipping it would lose
/// the error (or the `exit`) it carries.
pub fn skip_keeping_errors(
    iter: impl Iterator<Item = Value>,
    n: usize,
) -> impl Iterator<Item = Value> {
    iter.enumerate()
        .filter_map(move |(index, value)| (index >= n || value.is_error()).then_some(value))
}

/// Takes the item to report out of `items`, if any of them holds an error: an `exit` if there is
/// one, because that has to end the script, or else the first error.
pub fn take_error(items: &mut Vec<Value>) -> Option<Value> {
    let holds_exit = |item: &Value| matches!(item, Value::Error { error, .. } if error.is_exit());
    let index = items
        .iter()
        .position(holds_exit)
        .or_else(|| items.iter().position(Value::is_error))?;
    Some(items.swap_remove(index))
}

/// Makes a list out of `items`, unless one of them holds an error, which is returned instead.
///
/// This is for commands that group stream items. An error buried in a list wouldn't be noticed by
/// whatever collects the stream, and it would disappear with the list if the list were dropped.
/// The other items are lost with the group, which only a consumer that recovers from the error
/// can notice, since anything else stops at it.
pub fn list_or_error(mut items: Vec<Value>, span: Span) -> Value {
    take_error(&mut items).unwrap_or_else(|| Value::list(items, span))
}

/// What a predicate or key `closure` returns for `value`. A closure that returns an error fails
/// just like one that raises it.
pub fn closure_result(
    closure: &mut ClosureEval,
    value: &Value,
    span: Span,
) -> Result<Value, ShellError> {
    closure
        .run_with_value(value.clone())
        .and_then(|data| data.into_value(span))
        .and_then(Value::unwrap_error)
}

/// Takes items for as long as `keep_taking` accepts what `closure` returns for them, then up to
/// `include` more, counting the one that stopped it.
///
/// An item that holds an error is always taken, and a closure that fails has its error take the
/// item's place and stops the taking, so that neither one goes missing. Nothing is pulled from
/// `iter` once the taking is over, since that item would be dropped along with any error in it.
pub fn take_by_closure(
    mut iter: impl Iterator<Item = Value>,
    mut closure: ClosureEval,
    head: Span,
    keep_taking: fn(&Value) -> bool,
    include: usize,
) -> impl Iterator<Item = Value> {
    // How many more items to take, once the closure has stopped the taking.
    let mut remaining = None;

    std::iter::from_fn(move || {
        match &mut remaining {
            Some(0) => return None,
            Some(left) => {
                *left -= 1;
                return iter.next();
            }
            None => {}
        }

        let value = iter.next()?;
        if value.is_error() {
            return Some(value);
        }

        match closure_result(&mut closure, &value, head) {
            Ok(cond) if keep_taking(&cond) => Some(value),
            Ok(_) => {
                remaining = Some(include.saturating_sub(1));
                (include > 0).then_some(value)
            }
            Err(error) => {
                remaining = Some(include);
                Some(Value::error(error, head))
            }
        }
    })
}

/// Drops leading items for as long as `keep_skipping` accepts what `closure` returns for them.
///
/// An item that holds an error is kept, without ending the skipping, and a closure that fails has
/// its error take the item's place and ends the skipping, so that neither one goes missing.
pub fn skip_by_closure(
    iter: impl Iterator<Item = Value>,
    mut closure: ClosureEval,
    head: Span,
    keep_skipping: fn(&Value) -> bool,
) -> impl Iterator<Item = Value> {
    let mut skipping = true;

    iter.filter_map(move |value| {
        if !skipping || value.is_error() {
            return Some(value);
        }

        match closure_result(&mut closure, &value, head) {
            Ok(cond) if keep_skipping(&cond) => None,
            result => {
                skipping = false;
                Some(result.map_or_else(|error| Value::error(error, head), |_| value))
            }
        }
    })
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

    let mut closure = ClosureEval::try_new(engine_state, stack, closure, head)?;

    for value in input {
        engine_state.signals().check(&head)?;
        let pred = closure.run_with_value(value)?.into_value(head)?.is_true();

        if pred == accumulator {
            return Ok(Value::bool(accumulator, head).into_pipeline_data());
        }
    }

    Ok(Value::bool(!accumulator, head).into_pipeline_data())
}
