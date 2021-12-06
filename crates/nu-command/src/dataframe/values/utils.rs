use nu_protocol::{span as span_join, ShellError, Span, Spanned, Value};

// Converts a Vec<Value> to a Vec<String> with a Span marking the whole
// location of the columns for error referencing
pub(crate) fn convert_columns(
    columns: Vec<Value>,
    span: Span,
) -> Result<(Vec<Spanned<String>>, Span), ShellError> {
    // First column span
    let mut col_span = columns
        .get(0)
        .ok_or_else(|| {
            ShellError::SpannedLabeledError(
                "Empty column list".into(),
                "Empty list found for command".into(),
                span,
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
            _ => Err(ShellError::SpannedLabeledError(
                "Incorrect column format".into(),
                "Only string as column name".into(),
                span,
            )),
        })
        .collect::<Result<Vec<Spanned<String>>, _>>()?;

    Ok((res, col_span))
}
