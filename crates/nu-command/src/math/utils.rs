use core::slice;
use indexmap::IndexMap;
use nu_protocol::{
    IntoPipelineData, PipelineData, Range, ShellError, Signals, Span, Value, engine::Call,
};

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
    // If we are not dealing with Primitives, then perhaps we are dealing with a table
    // Create a key for each column name
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
                //Turns out we are not dealing with a table
                return mf(values, val.span(), name);
            }
        }
    }
    // The mathematical function operates over the columns of the table
    let mut column_totals = IndexMap::new();
    for (col_name, col_vals) in column_values {
        if let Ok(out) = mf(&col_vals, val_span, name) {
            column_totals.insert(col_name, out);
        }
    }
    if column_totals.keys().len() == 0 {
        return Err(ShellError::UnsupportedInput {
            msg: "Unable to give a result with this input".to_string(),
            input: "value originates from here".into(),
            msg_span: name,
            input_span: val_span,
        });
    }

    Ok(Value::record(column_totals.into_iter().collect(), name))
}

pub fn calculate(
    values: PipelineData,
    name: Span,
    mf: impl Fn(&[Value], Span, Span) -> Result<Value, ShellError>,
) -> Result<Value, ShellError> {
    // TODO implement spans for ListStream, thus negating the need for unwrap_or().
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
                    *val = mf(slice::from_ref(val), span, name)?;
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
