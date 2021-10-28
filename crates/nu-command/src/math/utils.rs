use nu_protocol::ast::Call;
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Span, Value};
use std::collections::HashMap;

pub type MathFunction = fn(values: &[Value], span: &Span) -> Result<Value, ShellError>;

pub fn run_with_function(
    call: &Call,
    input: PipelineData,
    mf: MathFunction,
) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
    let name = call.head;
    let res = calculate(input, name, mf);
    match res {
        Ok(v) => Ok(v.into_pipeline_data()),
        Err(e) => Err(e),
    }
}

fn helper_for_tables(
    values: PipelineData,
    name: Span,
    mf: MathFunction,
) -> Result<Value, ShellError> {
    // If we are not dealing with Primitives, then perhaps we are dealing with a table
    // Create a key for each column name
    let mut column_values = HashMap::new();
    for val in values {
        if let Value::Record { cols, vals, .. } = val {
            for (key, value) in cols.iter().zip(vals.iter()) {
                column_values
                    .entry(key.clone())
                    .and_modify(|v: &mut Vec<Value>| v.push(value.clone()))
                    .or_insert_with(|| vec![value.clone()]);
            }
        }
    }
    // The mathematical function operates over the columns of the table
    let mut column_totals = HashMap::new();
    for (col_name, col_vals) in column_values {
        if let Ok(out) = mf(&col_vals, &name) {
            column_totals.insert(col_name, out);
        }
    }
    if column_totals.keys().len() == 0 {
        return Err(ShellError::UnsupportedInput(
            "Unable to give a result with this input".to_string(),
            name,
        ));
    }
    let (cols, vals) = column_totals
        .into_iter()
        .fold((vec![], vec![]), |mut acc, (k, v)| {
            acc.0.push(k);
            acc.1.push(v);
            acc
        });

    Ok(Value::Record {
        cols,
        vals,
        span: name,
    })
}

pub fn calculate(values: PipelineData, name: Span, mf: MathFunction) -> Result<Value, ShellError> {
    match values {
        PipelineData::Stream(_) => helper_for_tables(values, name, mf),
        PipelineData::Value(Value::List { ref vals, .. }) => match &vals[..] {
            [Value::Record { .. }, _end @ ..] => helper_for_tables(values, name, mf),
            _ => mf(vals, &name),
        },
        PipelineData::Value(Value::Record { vals, cols, span }) => {
            let new_vals: Result<Vec<Value>, ShellError> =
                vals.into_iter().map(|val| mf(&[val], &name)).collect();
            match new_vals {
                Ok(vec) => Ok(Value::Record {
                    cols,
                    vals: vec,
                    span,
                }),
                Err(err) => Err(err),
            }
        }
        PipelineData::Value(val) => mf(&[val], &name),
    }
}
