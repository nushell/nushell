use core::slice;
use indexmap::IndexMap;
use nu_engine::CallExt;
use nu_protocol::{
    IntoPipelineData, PipelineData, Range, ShellError, Signals, Span, Value,
    ast::CellPath,
    engine::{Call, EngineState, Stack, StateWorkingSet},
};
use std::sync::Arc;

pub fn run_with_function(
    call: &Call,
    input: PipelineData,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError>,
) -> Result<PipelineData, ShellError> {
    let name = call.head;
    let res = calculate(input, name, mf);
    match res {
        Ok(v) => Ok(v.into_pipeline_data()),
        Err(e) => Err(e),
    }
}

fn helper_for_tables(
    values: &[Value],
    val_span: Span,
    name: Span,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError>,
) -> Result<Value, ShellError> {
    let mut column_values = IndexMap::new();
    for val in values {
        match val {
            Value::Record { val, .. } => {
                for (key, value) in &**val {
                    column_values
                        .entry(key.clone())
                        .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                        .or_insert_with(|| vec![value.clone()]);
                }
            }
            Value::Error { error, .. } => return Err(*error.clone()),
            _ => {
                return mf(values, val.span(), name);
            }
        }
    }
    let mut column_totals = IndexMap::new();
    for (col_name, col_vals) in column_values {
        column_totals.insert(col_name, mf(&col_vals, val_span, name)?);
    }

    Ok(Value::record(column_totals.into_iter().collect(), name))
}

pub fn calculate(
    values: PipelineData,
    name: Span,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError>,
) -> Result<Value, ShellError> {
    let span = values.span().unwrap_or(name);
    match values {
        PipelineData::ListStream(s, ..) => {
            helper_for_tables(&s.into_iter().collect::<Vec<Value>>(), span, name, mf)
        }
        PipelineData::Value(Value::List { ref vals, .. }, ..) => match &vals[..] {
            [Value::Record { .. }, _end @ ..] => helper_for_tables(
                vals,
                values.span().expect("PipelineData::value had no span"),
                name,
                mf,
            ),
            _ => mf(vals, span, name),
        },
        PipelineData::Value(Value::Record { val, .. }, ..) => {
            let mut record = val.into_owned();
            record
                .iter_mut()
                .try_for_each(|(_, val)| -> Result<(), ShellError> {
                    let result = match val {
                        Value::List { vals, .. } => mf(vals, span, name),
                        _ => mf(slice::from_ref(val), span, name),
                    };
                    *val = result?;
                    Ok(())
                })?;
            Ok(Value::record(record, span))
        }
        PipelineData::Value(Value::Range { val, .. }, ..) => {
            ensure_bounded(val.as_ref(), span, name)?;
            let new_vals: Result<Vec<Value>, ShellError> = val
                .into_range_iter(span, Signals::empty())
                .map(|val| mf(&[val], span, name))
                .collect();

            mf(&new_vals?, span, name)
        }
        PipelineData::Value(val, ..) => mf(&[val], span, name),
        PipelineData::Empty => Err(ShellError::PipelineEmpty { dst_span: name }),
        val => Err(ShellError::UnsupportedInput {
            msg: "Only ints, floats, lists, records, or ranges are supported".into(),
            input: "value originates from here".into(),
            msg_span: name,
            input_span: val
                .span()
                .expect("non-Empty non-ListStream PipelineData had no span"),
        }),
    }
}

/// Apply a reducing math function (`mf`) to the values under each cell path.
///
/// List-valued cells are reduced as a whole (e.g. average of the list). Errors
/// under a path are left alone. When `cell_paths` is empty, falls through to
/// [`run_with_function`].
pub fn run_with_function_and_cell_paths(
    call: &Call,
    input: PipelineData,
    cell_paths: Vec<CellPath>,
    signals: &Signals,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError> + Send + Sync + 'static,
) -> Result<PipelineData, ShellError> {
    if cell_paths.is_empty() {
        return run_with_function(call, input, mf);
    }

    let name = call.head;
    let span = input.span().unwrap_or(name);
    let mf = Arc::new(mf);

    input.map(
        move |mut v| {
            for path in &cell_paths {
                let mf = mf.clone();
                let r = v.update_cell_path(
                    &path.members,
                    Box::new(move |old| {
                        let result = match old {
                            Value::List { vals, .. } => mf(vals, span, name),
                            Value::Error { .. } => Ok(old.clone()),
                            other => mf(slice::from_ref(other), span, name),
                        };
                        match result {
                            Ok(val) => val,
                            Err(e) => Value::error(e, span),
                        }
                    }),
                );
                if let Err(error) = r {
                    return Value::error(error, span);
                }
            }
            v
        },
        signals,
    )
}

pub fn run_with_function_with_cell_paths(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError> + Send + Sync + 'static,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
    run_with_function_and_cell_paths(call, input, cell_paths, engine_state.signals(), mf)
}

pub fn run_with_function_with_cell_paths_const(
    working_set: &StateWorkingSet,
    call: &Call,
    input: PipelineData,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError> + Send + Sync + 'static,
) -> Result<PipelineData, ShellError> {
    let cell_paths: Vec<CellPath> = call.rest_const(working_set, 0)?;
    run_with_function_and_cell_paths(
        call,
        input,
        cell_paths,
        working_set.permanent().signals(),
        mf,
    )
}

/// Apply an element-wise operation (`op`) across pipeline input.
///
/// - With cell paths: update those columns, mapping list cells element-wise.
/// - With a record and no cell paths: map every column element-wise (lists keep
///   their shape; scalars are transformed in place).
/// - Otherwise: optionally reject empty input, ensure ranges are bounded, then
///   map `op` over the pipeline stream.
pub fn run_with_elementwise(
    input: PipelineData,
    cell_paths: Vec<CellPath>,
    head: Span,
    signals: &Signals,
    reject_empty: bool,
    op: impl Fn(Value) -> Value + Send + Sync + 'static,
) -> Result<PipelineData, ShellError> {
    if !cell_paths.is_empty() {
        let op = Arc::new(op);
        return input.map(
            move |mut v| {
                for path in &cell_paths {
                    let op = op.clone();
                    let r = v.update_cell_path(
                        &path.members,
                        Box::new(move |old| map_elementwise(old, head, op.as_ref())),
                    );
                    if let Err(error) = r {
                        return Value::error(error, head);
                    }
                }
                v
            },
            signals,
        );
    }

    if let PipelineData::Value(Value::Record { val, .. }, metadata) = input {
        let mut record = val.into_owned();
        for (_, col_val) in record.iter_mut() {
            *col_val = map_elementwise(col_val, head, &op);
        }
        return Ok(PipelineData::Value(Value::record(record, head), metadata));
    }

    if reject_empty && matches!(input, PipelineData::Empty) {
        return Err(ShellError::PipelineEmpty { dst_span: head });
    }

    if let PipelineData::Value(ref v @ Value::Range { ref val, .. }, ..) = input {
        ensure_bounded(val, v.span(), head)?;
    }

    input.map(op, signals)
}

/// Map `op` over a list value element-wise; otherwise apply `op` once.
/// Errors are propagated unchanged.
fn map_elementwise(value: &Value, head: Span, op: &impl Fn(Value) -> Value) -> Value {
    match value {
        Value::Error { .. } => value.clone(),
        Value::List { vals, .. } => Value::list(vals.iter().map(|v| op(v.clone())).collect(), head),
        other => op(other.clone()),
    }
}

pub fn ensure_bounded(range: &Range, val_span: Span, call_span: Span) -> Result<(), ShellError> {
    if range.is_bounded() {
        return Ok(());
    }
    Err(ShellError::IncorrectValue {
        msg: "Range must be bounded".to_string(),
        val_span,
        call_span,
    })
}

/// Expand range input into a concrete list, erroring if the range is unbounded.
pub fn expand_range_input(
    input: PipelineData,
    call_span: Span,
) -> Result<PipelineData, ShellError> {
    let span = input.span().unwrap_or(call_span);
    match input.try_expand_range() {
        Ok(val) => Ok(val),
        Err(_) => Err(ShellError::IncorrectValue {
            msg: "Range must be bounded".to_string(),
            val_span: span,
            call_span,
        }),
    }
}
