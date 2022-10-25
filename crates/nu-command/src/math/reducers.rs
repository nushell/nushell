use nu_protocol::{ShellError, Span, Value};
use std::cmp::Ordering;

pub enum Reduce {
    Summation,
    Product,
    Minimum,
    Maximum,
}

pub type ReducerFunction =
    Box<dyn Fn(Value, Vec<Value>, Span) -> Result<Value, ShellError> + Send + Sync + 'static>;

pub fn reducer_for(command: Reduce) -> ReducerFunction {
    match command {
        Reduce::Summation => Box::new(|_, values, head| sum(values, head)),
        Reduce::Product => Box::new(|_, values, head| product(values, head)),
        Reduce::Minimum => Box::new(|_, values, head| min(values, head)),
        Reduce::Maximum => Box::new(|_, values, head| max(values, head)),
    }
}

pub fn max(data: Vec<Value>, head: Span) -> Result<Value, ShellError> {
    let mut biggest = data
        .first()
        .ok_or_else(|| ShellError::UnsupportedInput("Empty input".to_string(), head))?
        .clone();

    for value in &data {
        if let Some(result) = value.partial_cmp(&biggest) {
            if result == Ordering::Greater {
                biggest = value.clone();
            }
        } else {
            return Err(ShellError::OperatorMismatch {
                op_span: head,
                lhs_ty: biggest.get_type(),
                lhs_span: biggest.span()?,
                rhs_ty: value.get_type(),
                rhs_span: value.span()?,
            });
        }
    }
    Ok(biggest)
}

pub fn min(data: Vec<Value>, head: Span) -> Result<Value, ShellError> {
    let mut smallest = data
        .first()
        .ok_or_else(|| ShellError::UnsupportedInput("Empty input".to_string(), head))?
        .clone();

    for value in &data {
        if let Some(result) = value.partial_cmp(&smallest) {
            if result == Ordering::Less {
                smallest = value.clone();
            }
        } else {
            return Err(ShellError::OperatorMismatch {
                op_span: head,
                lhs_ty: smallest.get_type(),
                lhs_span: smallest.span()?,
                rhs_ty: value.get_type(),
                rhs_span: value.span()?,
            });
        }
    }
    Ok(smallest)
}

pub fn sum(data: Vec<Value>, head: Span) -> Result<Value, ShellError> {
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
            head,
        )),
        _ => Ok(Value::nothing(head)),
    }?;

    for value in &data {
        match value {
            Value::Int { .. }
            | Value::Float { .. }
            | Value::Filesize { .. }
            | Value::Duration { .. } => {
                acc = acc.add(head, value, head)?;
            }
            other => {
                return Err(ShellError::UnsupportedInput(
                    "Attempted to compute the sum of a value that cannot be summed".to_string(),
                    other.span().unwrap_or(head),
                ));
            }
        }
    }
    Ok(acc)
}

pub fn product(data: Vec<Value>, head: Span) -> Result<Value, ShellError> {
    let initial_value = data.get(0);

    let mut acc = match initial_value {
        Some(Value::Int { span, .. }) | Some(Value::Float { span, .. }) => Ok(Value::Int {
            val: 1,
            span: *span,
        }),
        None => Err(ShellError::UnsupportedInput(
            "Empty input".to_string(),
            head,
        )),
        _ => Ok(Value::nothing(head)),
    }?;

    for value in &data {
        match value {
            Value::Int { .. } | Value::Float { .. } => {
                acc = acc.mul(head, value, head)?;
            }
            other => {
                return Err(ShellError::UnsupportedInput(
                    "Attempted to compute the product of a value that cannot be multiplied"
                        .to_string(),
                    other.span().unwrap_or(head),
                ));
            }
        }
    }
    Ok(acc)
}
