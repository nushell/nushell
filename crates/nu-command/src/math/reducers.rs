use nu_protocol::{ShellError, Span, SpanId, Value};
use std::cmp::Ordering;

pub enum Reduce {
    Summation,
    Product,
    Minimum,
    Maximum,
}

pub type ReducerFunction =
    Box<dyn Fn(Value, Vec<Value>, Span, SpanId, Span, SpanId) -> Result<Value, ShellError> + Send + Sync + 'static>;

pub fn reducer_for(command: Reduce) -> ReducerFunction {
    match command {
        Reduce::Summation => Box::new(|_, values, span, span_id, head, head_id| sum(values, span, head, head_id)),
        Reduce::Product => Box::new(|_, values, span, span_id, head, head_id| product(values, span, head, head_id)),
        Reduce::Minimum => Box::new(|_, values, span, span_id, head, head_id| min(values, span, head)),
        Reduce::Maximum => Box::new(|_, values, span, span_id, head, head_id| max(values, span, head)),
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
        if let Some(result) = value.partial_cmp(&biggest) {
            if result == Ordering::Greater {
                biggest = value.clone();
            }
        } else {
            return Err(ShellError::OperatorMismatch {
                op_span: head,
                lhs_ty: biggest.get_type().to_string(),
                lhs_span: biggest.span(),
                rhs_ty: value.get_type().to_string(),
                rhs_span: value.span(),
            });
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
        if let Some(result) = value.partial_cmp(&smallest) {
            if result == Ordering::Less {
                smallest = value.clone();
            }
        } else {
            return Err(ShellError::OperatorMismatch {
                op_span: head,
                lhs_ty: smallest.get_type().to_string(),
                lhs_span: smallest.span(),
                rhs_ty: value.get_type().to_string(),
                rhs_span: value.span(),
            });
        }
    }
    Ok(smallest)
}

pub fn sum(data: Vec<Value>, span: Span, head: Span, head_id: SpanId) -> Result<Value, ShellError> {
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
                acc = acc.add(head, head_id, value, head)?;
            }
            Value::Error { error, .. } => return Err(*error.clone()),
            other => {
                return Err(ShellError::UnsupportedInput {
                    msg: "Attempted to compute the sum of a value that cannot be summed"
                        .to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: other.span(),
                });
            }
        }
    }
    Ok(acc)
}

pub fn product(data: Vec<Value>, span: Span, head: Span, head_id: SpanId) -> Result<Value, ShellError> {
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
                acc = acc.mul(head, head_id, value, head)?;
            }
            Value::Error { error, .. } => return Err(*error.clone()),
            other => {
                return Err(ShellError::UnsupportedInput {
                    msg: "Attempted to compute the product of a value that cannot be multiplied"
                        .to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: other.span(),
                });
            }
        }
    }
    Ok(acc)
}
