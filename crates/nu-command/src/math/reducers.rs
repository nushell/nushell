use nu_protocol::{ShellError, Span, Value};
use std::cmp::Ordering;

pub enum Reduce {
    Summation,
    Product,
    Minimum,
    Maximum,
}

pub type ReducerFunction =
    Box<dyn Fn(Value, Vec<Value>, Span, Span) -> Result<Value, ShellError> + Send + Sync + 'static>;

pub fn reducer_for(command: Reduce) -> ReducerFunction {
    match command {
        Reduce::Summation => Box::new(|_, values, span, head| sum(values, span, head)),
        Reduce::Product => Box::new(|_, values, span, head| product(values, span, head)),
        Reduce::Minimum => Box::new(|_, values, span, head| min(values, span, head)),
        Reduce::Maximum => Box::new(|_, values, span, head| max(values, span, head)),
    }
}

pub fn max(data: Vec<Value>, span: Span, head: Span) -> Result<Value, ShellError> {
    let mut biggest = data
        .first()
        .ok_or_else(|| ShellError::UnsupportedInput {
            msg: "Empty input".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        })?
        .clone();

    for value in &data {
        if value.partial_cmp(&biggest) == Some(Ordering::Greater) {
            biggest = value.clone();
        }
    }
    Ok(biggest)
}

pub fn min(data: Vec<Value>, span: Span, head: Span) -> Result<Value, ShellError> {
    let mut smallest = data
        .first()
        .ok_or_else(|| ShellError::UnsupportedInput {
            msg: "Empty input".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        })?
        .clone();

    for value in &data {
        if value.partial_cmp(&smallest) == Some(Ordering::Less) {
            smallest = value.clone();
        }
    }
    Ok(smallest)
}

pub fn sum(data: Vec<Value>, span: Span, head: Span) -> Result<Value, ShellError> {
    let initial_value = data.first();

    let mut acc = match initial_value {
        Some(v) => {
            let span = v.span();
            match v {
                Value::Filesize { .. } => Ok(Value::filesize(0, span)),
                Value::Duration { .. } => Ok(Value::duration(0, span)),
                Value::Int { .. } | Value::Float { .. } => Ok(Value::int(0, span)),
                _ => Ok(Value::nothing(head)),
            }
        }

        None => Err(ShellError::UnsupportedInput {
            msg: "Empty input".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        }),
    }?;

    for value in &data {
        match value {
            Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. } => {
                acc = acc.add(head, value, head)?;
            }
            Value::Error { error, .. } => return Err(*error.clone()),
            other => {
                return Err(ShellError::UnsupportedInput {
                    msg: format!("Attempted to compute the sum of a value '{}' that cannot be summed with a type of `{}`.",
                        other.coerce_string()?, other.get_type()),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: other.span(),
                });
            }
        }
    }
    Ok(acc)
}

pub fn product(data: Vec<Value>, span: Span, head: Span) -> Result<Value, ShellError> {
    let initial_value = data.first();

    let mut acc = match initial_value {
        Some(v) => {
            let span = v.span();
            match v {
                Value::Int { .. } | Value::Float { .. } => Ok(Value::int(1, span)),
                _ => Ok(Value::nothing(head)),
            }
        }
        None => Err(ShellError::UnsupportedInput {
            msg: "Empty input".to_string(),
            input: "value originates from here".into(),
            msg_span: head,
            input_span: span,
        }),
    }?;

    for value in &data {
        match value {
            Value::Int { .. } | Value::Float { .. } => {
                acc = acc.mul(head, value, head)?;
            }
            Value::Error { error, .. } => return Err(*error.clone()),
            other => {
                return Err(ShellError::UnsupportedInput {
                    msg: format!("Attempted to compute the product of a value '{}' that cannot be multiplied with a type of `{}`.",
                        other.coerce_string()?, other.get_type()),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: other.span(),
                });
            }
        }
    }
    Ok(acc)
}
