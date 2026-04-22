use nu_protocol::shell_error::generic::GenericError;
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
        .ok_or_else(|| {
            ShellError::Generic(GenericError::new(
                "Empty column list",
                "Empty list found for command",
                span,
            ))
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
                _ => Err(ShellError::Generic(GenericError::new(
                    "Incorrect column format",
                    "Only string as column name",
                    span,
                ))),
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
        .ok_or_else(|| {
            ShellError::Generic(GenericError::new(
                "Empty column list",
                "Empty list found for command",
                span,
            ))
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
                _ => Err(ShellError::Generic(GenericError::new(
                    "Incorrect column format",
                    "Only string as column name",
                    span,
                ))),
            }
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok((res, col_span))
}
