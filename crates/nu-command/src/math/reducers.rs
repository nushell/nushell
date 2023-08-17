use nu_protocol::{ShellError, Span, SpannedValue};
use std::cmp::Ordering;

pub enum Reduce {
    Summation,
    Product,
    Minimum,
    Maximum,
}

pub type ReducerFunction = Box<
    dyn Fn(SpannedValue, Vec<SpannedValue>, Span, Span) -> Result<SpannedValue, ShellError>
        + Send
        + Sync
        + 'static,
>;

pub fn reducer_for(command: Reduce) -> ReducerFunction {
    match command {
        Reduce::Summation => Box::new(|_, values, span, head| sum(values, span, head)),
        Reduce::Product => Box::new(|_, values, span, head| product(values, span, head)),
        Reduce::Minimum => Box::new(|_, values, span, head| min(values, span, head)),
        Reduce::Maximum => Box::new(|_, values, span, head| max(values, span, head)),
    }
}

pub fn max(data: Vec<SpannedValue>, span: Span, head: Span) -> Result<SpannedValue, ShellError> {
    let mut biggest = data
        .first()
        .ok_or_else(|| {
            ShellError::UnsupportedInput(
                "Empty input".to_string(),
                "value originates from here".into(),
                head,
                span,
            )
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
                lhs_span: biggest.span()?,
                rhs_ty: value.get_type().to_string(),
                rhs_span: value.span()?,
            });
        }
    }
    Ok(biggest)
}

pub fn min(data: Vec<SpannedValue>, span: Span, head: Span) -> Result<SpannedValue, ShellError> {
    let mut smallest = data
        .first()
        .ok_or_else(|| {
            ShellError::UnsupportedInput(
                "Empty input".to_string(),
                "value originates from here".into(),
                head,
                span,
            )
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
                lhs_span: smallest.span()?,
                rhs_ty: value.get_type().to_string(),
                rhs_span: value.span()?,
            });
        }
    }
    Ok(smallest)
}

pub fn sum(data: Vec<SpannedValue>, span: Span, head: Span) -> Result<SpannedValue, ShellError> {
    let initial_value = data.get(0);

    let mut acc = match initial_value {
        Some(SpannedValue::Filesize { span, .. }) => Ok(SpannedValue::Filesize {
            val: 0,
            span: *span,
        }),
        Some(SpannedValue::Duration { span, .. }) => Ok(SpannedValue::Duration {
            val: 0,
            span: *span,
        }),
        Some(SpannedValue::Int { span, .. }) | Some(SpannedValue::Float { span, .. }) => {
            Ok(SpannedValue::int(0, *span))
        }
        None => Err(ShellError::UnsupportedInput(
            "Empty input".to_string(),
            "value originates from here".into(),
            head,
            span,
        )),
        _ => Ok(SpannedValue::nothing(head)),
    }?;

    for value in &data {
        match value {
            SpannedValue::Int { .. }
            | SpannedValue::Float { .. }
            | SpannedValue::Filesize { .. }
            | SpannedValue::Duration { .. } => {
                acc = acc.add(head, value, head)?;
            }
            SpannedValue::Error { error, .. } => return Err(*error.clone()),
            other => {
                return Err(ShellError::UnsupportedInput(
                    "Attempted to compute the sum of a value that cannot be summed".to_string(),
                    "value originates from here".into(),
                    head,
                    other.span(),
                ));
            }
        }
    }
    Ok(acc)
}

pub fn product(
    data: Vec<SpannedValue>,
    span: Span,
    head: Span,
) -> Result<SpannedValue, ShellError> {
    let initial_value = data.get(0);

    let mut acc = match initial_value {
        Some(SpannedValue::Int { span, .. }) | Some(SpannedValue::Float { span, .. }) => {
            Ok(SpannedValue::int(1, *span))
        }
        None => Err(ShellError::UnsupportedInput(
            "Empty input".to_string(),
            "value originates from here".into(),
            head,
            span,
        )),
        _ => Ok(SpannedValue::nothing(head)),
    }?;

    for value in &data {
        match value {
            SpannedValue::Int { .. } | SpannedValue::Float { .. } => {
                acc = acc.mul(head, value, head)?;
            }
            SpannedValue::Error { error, .. } => return Err(*error.clone()),
            other => {
                return Err(ShellError::UnsupportedInput(
                    "Attempted to compute the product of a value that cannot be multiplied"
                        .to_string(),
                    "value originates from here".into(),
                    head,
                    other.span(),
                ));
            }
        }
    }
    Ok(acc)
}
