use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, UntaggedValue, Value};

// Converts a Vec<Value> to a Vec<String> with a Span marking the whole
// location of the columns for error referencing
pub(crate) fn convert_columns<'columns>(
    columns: &'columns [Value],
    tag: &Tag,
) -> Result<(Vec<&'columns str>, Span), ShellError> {
    let mut col_span = match columns
        .iter()
        .nth(0)
        .map(|v| Span::new(v.tag.span.start(), v.tag.span.end()))
    {
        Some(span) => span,
        None => {
            return Err(ShellError::labeled_error(
                "Empty column list",
                "Empty list found for command",
                tag,
            ))
        }
    };

    let res = columns
        .iter()
        .map(|value| match &value.value {
            UntaggedValue::Primitive(Primitive::String(s)) => {
                col_span = col_span.until(value.tag.span);
                Ok(s.as_ref())
            }
            _ => Err(ShellError::labeled_error(
                "Incorrect column format",
                "Only string as column name",
                &value.tag,
            )),
        })
        .collect::<Result<Vec<&'columns str>, _>>()?;

    Ok((res, col_span))
}
