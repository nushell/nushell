use nu_protocol::{ShellError, Span, Spanned, Value};

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
        .first()
        .ok_or_else(|| ShellError::GenericError {
            error: "Empty column list".into(),
            msg: "Empty list found for command".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?
        .span();

    let res = columns
        .into_iter()
        .map(|value| {
            let span = value.span();
            match value {
                Value::String { val, .. } => {
                    col_span = col_span.merge(span);
                    Ok(Spanned { item: val, span })
                }
                _ => Err(ShellError::GenericError {
                    error: "Incorrect column format".into(),
                    msg: "Only string as column name".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }),
            }
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
        .first()
        .ok_or_else(|| ShellError::GenericError {
            error: "Empty column list".into(),
            msg: "Empty list found for command".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        })
        .map(|v| v.span())?;

    let res = columns
        .into_iter()
        .map(|value| {
            let span = value.span();
            match value {
                Value::String { val, .. } => {
                    col_span = col_span.merge(span);
                    Ok(val)
                }
                _ => Err(ShellError::GenericError {
                    error: "Incorrect column format".into(),
                    msg: "Only string as column name".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }),
            }
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok((res, col_span))
}
