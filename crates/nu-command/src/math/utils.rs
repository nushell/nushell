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

/// Reduce a collected list or stream of values.
///
/// Tables (first item is a record) are reduced per column via [`helper_for_tables`].
/// Empty collections and non-record lists go through `mf` directly so empty
/// streams match empty lists (e.g. both error with "Empty input" for `math max`).
fn reduce_collected_values(
    vals: &[Value],
    val_span: Span,
    name: Span,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError>,
) -> Result<Value, ShellError> {
    match vals {
        [Value::Record { .. }, ..] => helper_for_tables(vals, val_span, name, mf),
        _ => mf(vals, val_span, name),
    }
}

pub fn calculate(
    values: PipelineData,
    name: Span,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError>,
) -> Result<Value, ShellError> {
    let span = values.span().unwrap_or(name);
    match values {
        PipelineData::ListStream(s, ..) => {
            let vals = s.into_iter().collect::<Vec<Value>>();
            reduce_collected_values(&vals, span, name, mf)
        }
        PipelineData::Value(Value::List { ref vals, .. }, ..) => {
            reduce_collected_values(vals, span, name, mf)
        }
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

/// Apply a reducing math function (`mf`) under each cell path, **per pipeline item**.
///
/// List-valued cells are reduced as a whole (e.g. average of the list). Scalar
/// cells are reduced as a one-element list. Tables are handled row-by-row (like
/// `input_handler::operate`), not by collecting a whole column and writing the
/// same result into every row.
///
/// When `cell_paths` is empty, falls through to [`run_with_function`].
///
/// Path update failures and math errors surface as top-level [`ShellError`] for a
/// single concrete value (`PipelineData::map` converts `Value::Error`), and as
/// in-band `Value::Error` items in streams.
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

    // Always map per pipeline item so tables (list of records) update each row
    // independently rather than reducing the whole column once.
    input.map(
        move |mut v| {
            for path in &cell_paths {
                let mf = mf.clone();
                let r = v.update_cell_path(
                    &path.members,
                    Box::new(move |old| {
                        let result = match old {
                            Value::List { vals, .. } => mf(vals, span, name),
                            other => mf(slice::from_ref(other), span, name),
                        };
                        match result {
                            Ok(val) => val,
                            // `update_cell_path` turns this into `Err`, which we
                            // re-embed below so stream rows stay in-band.
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
/// - With cell paths: update those columns **per pipeline item** (row-by-row for
///   tables), mapping list cells element-wise.
/// - With a record and no cell paths: map every column element-wise (lists keep
///   their shape; scalars are transformed in place).
/// - Otherwise: optionally reject empty input, ensure ranges are bounded, then
///   map `op` over the pipeline stream.
///
/// For a single concrete record (no cell paths), errors from `op` are returned as
/// top-level [`ShellError`]. `PipelineData::map` also promotes a single
/// `Value::Error` to a top-level error; stream rows keep errors in-band.
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
                        Box::new(move |old| match map_elementwise(old, op.as_ref()) {
                            Ok(val) => val,
                            Err(e) => Value::error(e, head),
                        }),
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

    if let PipelineData::Value(
        Value::Record {
            val, internal_span, ..
        },
        metadata,
    ) = input
    {
        let mut record = val.into_owned();
        for (_, col_val) in record.iter_mut() {
            *col_val = map_elementwise(col_val, &op)?;
        }
        return Ok(PipelineData::Value(
            Value::record(record, internal_span),
            metadata,
        ));
    }

    if reject_empty && matches!(input, PipelineData::Empty) {
        return Err(ShellError::PipelineEmpty { dst_span: head });
    }

    if let PipelineData::Value(ref v @ Value::Range { ref val, .. }, ..) = input {
        ensure_bounded(val, v.span(), head)?;
    }

    // `PipelineData::map` maps list/range elements, and turns a single
    // `Value::Error` into a top-level `ShellError`.
    input.map(op, signals)
}

/// Map `op` over a list value element-wise; otherwise apply `op` once.
/// Errors from `op` (or nested [`Value::Error`]s) are returned as [`ShellError`].
fn map_elementwise(value: &Value, op: &impl Fn(Value) -> Value) -> Result<Value, ShellError> {
    match value {
        Value::Error { error, .. } => Err(*error.clone()),
        Value::List { vals, .. } => {
            let list_span = value.span();
            let mut out = Vec::with_capacity(vals.len());
            for v in vals {
                match op(v.clone()) {
                    Value::Error { error, .. } => return Err(*error),
                    other => out.push(other),
                }
            }
            Ok(Value::list(out, list_span))
        }
        other => match op(other.clone()) {
            Value::Error { error, .. } => Err(*error),
            result => Ok(result),
        },
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

/// Shared expected-type string for math commands that accept numeric-like values
/// (int, float, filesize, duration).
pub const NUMERIC_INPUT_TYPES: &str = "int, float, filesize, or duration";

/// Expected-type string for math commands that only accept plain numbers.
pub const NUMBER_INPUT_TYPES: &str = "int or float";

/// Classify the unit of a math-compatible numeric value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericUnit {
    Number,
    Duration,
    Filesize,
}

impl NumericUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            NumericUnit::Number => "int or float",
            NumericUnit::Duration => "duration",
            NumericUnit::Filesize => "filesize",
        }
    }
}

/// Convert a math-compatible value to an `f64` for statistical calculations,
/// reporting its unit kind. Int/float share [`NumericUnit::Number`].
pub fn to_unit_f64(value: &Value, head: Span) -> Result<(NumericUnit, f64), ShellError> {
    match value {
        Value::Int { val, .. } => Ok((NumericUnit::Number, *val as f64)),
        Value::Float { val, .. } => Ok((NumericUnit::Number, *val)),
        Value::Duration { val, .. } => Ok((NumericUnit::Duration, *val as f64)),
        Value::Filesize { val, .. } => Ok((NumericUnit::Filesize, val.get() as f64)),
        Value::Error { error, .. } => Err(*error.clone()),
        other => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: NUMERIC_INPUT_TYPES.into(),
            wrong_type: other.get_type().to_string(),
            dst_span: head,
            src_span: other.span(),
        }),
    }
}

/// Re-wrap a computed numeric result into the unit of the original data.
///
/// For [`NumericUnit::Number`] the raw float is returned (as float). For duration
/// and filesize the value is rounded to the nearest integer unit.
pub fn wrap_unit_f64(unit: NumericUnit, val: f64, span: Span) -> Value {
    match unit {
        NumericUnit::Number => Value::float(val, span),
        NumericUnit::Duration => Value::duration(val.round() as i64, span),
        NumericUnit::Filesize => Value::filesize(val.round() as i64, span),
    }
}

/// Shared empty / sample-size checks for variance-like reducers.
pub fn variance_denominator(
    n: usize,
    sample: bool,
    head: Span,
    input_span: Span,
) -> Result<usize, ShellError> {
    if n == 0 {
        return Err(ShellError::UnsupportedInput {
            msg: "Empty input".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span,
        });
    }
    if sample {
        if n < 2 {
            return Err(ShellError::UnsupportedInput {
                msg: "Sample variance requires at least 2 values".to_string(),
                input: "value originates from here".into(),
                msg_span: head,
                input_span,
            });
        }
        Ok(n - 1)
    } else {
        Ok(n)
    }
}
