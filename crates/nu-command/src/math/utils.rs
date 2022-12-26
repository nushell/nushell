use indexmap::map::IndexMap;
use nu_protocol::ast::Call;
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Span, Spanned, Value};

pub fn run_with_function(
    call: &Call,
    input: PipelineData,
    mf: impl Fn(&[Value], Span, &Span) -> Result<Value, ShellError>,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
    mf: impl Fn(&[Value], Span, &Span) -> Result<Value, ShellError>,
) -> Result<Value, ShellError> {
    // If we are not dealing with Primitives, then perhaps we are dealing with a table
    // Create a key for each column name
    let mut column_values = IndexMap::new();
    for val in values {
        match val {
            Value::Record { cols, vals, .. } => {
                for (key, value) in cols.iter().zip(vals.iter()) {
                    column_values
                        .entry(key.clone())
                        .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                        .or_insert_with(|| vec![value.clone()]);
                }
            }
            Value::Error { error } => return Err(error.clone()),
            _ => {
                //Turns out we are not dealing with a table
                return mf(values, val.expect_span(), &name);
            }
        }
    }
    // The mathematical function operates over the columns of the table
    let mut column_totals = IndexMap::new();
    for (col_name, col_vals) in column_values {
        if let Ok(out) = mf(&col_vals, val_span, &name) {
            column_totals.insert(col_name, out);
        }
    }
    if column_totals.keys().len() == 0 {
        return Err(ShellError::UnsupportedInput(
            "Unable to give a result with this input".to_string(),
            "value originates from here".into(),
            name,
            val_span,
        ));
    }

    Ok(Value::from(Spanned {
        item: column_totals,
        span: name,
    }))
}

pub fn calculate(
    values: PipelineData,
    name: Span,
    mf: impl Fn(&[Value], Span, &Span) -> Result<Value, ShellError>,
) -> Result<Value, ShellError> {
    // TODO implement spans for ListStream, thus negating the need for unwrap_or().
    let span = values.span().unwrap_or(name);
    match values {
        PipelineData::ListStream(s, ..) => {
            helper_for_tables(&s.collect::<Vec<Value>>(), span, name, mf)
        }
        PipelineData::Value(Value::List { ref vals, span }, ..) => match &vals[..] {
            [Value::Record { .. }, _end @ ..] => helper_for_tables(
                vals,
                values.span().expect("PipelineData::Value had no span"),
                name,
                mf,
            ),
            _ => mf(vals, span, &name),
        },
        PipelineData::Value(Value::Record { vals, cols, span }, ..) => {
            let new_vals: Result<Vec<Value>, ShellError> = vals
                .into_iter()
                .map(|val| mf(&[val], span, &name))
                .collect();
            match new_vals {
                Ok(vec) => Ok(Value::Record {
                    cols,
                    vals: vec,
                    span,
                }),
                Err(err) => Err(err),
            }
        }
        PipelineData::Value(Value::Range { val, span, .. }, ..) => {
            let new_vals: Result<Vec<Value>, ShellError> = val
                .into_range_iter(None)?
                .map(|val| mf(&[val], span, &name))
                .collect();

            mf(&new_vals?, span, &name)
        }
        PipelineData::Value(val, ..) => mf(&[val], span, &name),
        PipelineData::Empty { .. } => Err(ShellError::PipelineEmpty(name)),
        val => Err(ShellError::UnsupportedInput(
            "Only integers, floats, lists, records or ranges are supported".into(),
            "value originates from here".into(),
            name,
            // This requires both the ListStream and Empty match arms to be above it.
            val.span()
                .expect("non-Empty non-ListStream PipelineData had no span"),
        )),
    }
}
