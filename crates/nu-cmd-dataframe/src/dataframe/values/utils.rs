use nu_protocol::{span as span_join, ShellError, Span, Spanned, Value};

// Default value used when selecting rows from dataframe
pub const DEFAULT_ROWS: usize = 5;

// Converts a Vec<Value> to a Vec<Spanned<String>> with a Span marking the whole
// location of the columns for error referencing
pub(crate) fn convert_columns(
    columns: Vec<Value>,
    span: Span,
) -> Result<(Vec<Spanned<String>>, Span), ShellError> {
    // First column span
    let mut col_span = columns
        .get(0)
        .ok_or_else(|| {
            ShellError::GenericError(
                "Empty column list".into(),
                "Empty list found for command".into(),
                Some(span),
                None,
                Vec::new(),
            )
        })
        .and_then(|v| v.span())?;

    let res = columns
        .into_iter()
        .map(|value| match value {
            Value::String { val, span } => {
                col_span = span_join(&[col_span, span]);
                Ok(Spanned { item: val, span })
            }
            _ => Err(ShellError::GenericError(
                "Incorrect column format".into(),
                "Only string as column name".into(),
                Some(span),
                None,
                Vec::new(),
            )),
        })
        .collect::<Result<Vec<Spanned<String>>, _>>()?;

    Ok((res, col_span))
}

// Converts a Vec<Value> to a Vec<String> with a Span marking the whole
// location of the columns for error referencing
pub(crate) fn convert_columns_string(
    columns: Vec<Value>,
    span: Span,
) -> Result<(Vec<String>, Span), ShellError> {
    // First column span
    let mut col_span = columns
        .get(0)
        .ok_or_else(|| {
            ShellError::GenericError(
                "Empty column list".into(),
                "Empty list found for command".into(),
                Some(span),
                None,
                Vec::new(),
            )
        })
        .and_then(|v| v.span())?;

    let res = columns
        .into_iter()
        .map(|value| match value {
            Value::String { val, span } => {
                col_span = span_join(&[col_span, span]);
                Ok(val)
            }
            _ => Err(ShellError::GenericError(
                "Incorrect column format".into(),
                "Only string as column name".into(),
                Some(span),
                None,
                Vec::new(),
            )),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok((res, col_span))
}
