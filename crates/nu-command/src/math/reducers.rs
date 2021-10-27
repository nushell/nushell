use nu_protocol::{ShellError, Span, Value};

#[allow(dead_code)]
pub enum Reduce {
    Summation,
}

pub fn reducer_for(
    command: Reduce,
) -> Box<dyn Fn(Value, Vec<Value>) -> Result<Value, ShellError> + Send + Sync + 'static> {
    match command {
        Reduce::Summation => Box::new(|_, values| sum(values)),
    }
}

pub fn sum(data: Vec<Value>) -> Result<Value, ShellError> {
    let initial_value = data.get(0);

    let mut acc = match initial_value {
        Some(Value::Filesize { span, .. }) => Ok(Value::Filesize {
            val: 0,
            span: *span,
        }),
        Some(Value::Duration { span, .. }) => Ok(Value::Duration {
            val: 0,
            span: *span,
        }),
        Some(Value::Int { span, .. }) | Some(Value::Float { span, .. }) => Ok(Value::Int {
            val: 0,
            span: *span,
        }),
        None => Err(ShellError::UnsupportedInput(
            "Empty input".to_string(),
            Span::unknown(),
        )),
        _ => Ok(Value::nothing()),
    }?;

    for value in &data {
        match value {
            Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. } => {
                let new_value = acc.add(acc.span().unwrap_or_else(|_| Span::unknown()), value);
                if new_value.is_err() {
                    return new_value;
                }
                acc = new_value.expect("This should never trigger")
            }
            other => {
                return Err(ShellError::UnsupportedInput(
                    "Attempted to compute the sum of a value that cannot be summed".to_string(),
                    other.span().unwrap_or_else(|_| Span::unknown()),
                ));
            }
        }
    }
    Ok(acc)
}
