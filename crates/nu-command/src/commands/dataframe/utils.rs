use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, UntaggedValue, Value};
use polars::prelude::PolarsError;

// Converts a Vec<Value> to a Vec<String> with a Span marking the whole
// location of the columns for error referencing
pub(crate) fn convert_columns(
    columns: &[Value],
    tag: &Tag,
) -> Result<(Vec<String>, Span), ShellError> {
    let mut col_span = match columns
        .get(0)
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
                Ok(s.clone())
            }
            _ => Err(ShellError::labeled_error(
                "Incorrect column format",
                "Only string as column name",
                &value.tag,
            )),
        })
        .collect::<Result<Vec<String>, _>>()?;

    Ok((res, col_span))
}

pub(crate) fn parse_polars_error<T: AsRef<str>>(
    e: &PolarsError,
    span: &Span,
    secondary: Option<T>,
) -> ShellError {
    let msg = match e {
        PolarsError::PolarsArrowError(_) => "PolarsArrow Error",
        PolarsError::ArrowError(_) => "Arrow Error",
        PolarsError::InvalidOperation(_) => "Invalid Operation",
        PolarsError::DataTypeMisMatch(_) => "Data Type Mismatch",
        PolarsError::NotFound(_) => "Not Found",
        PolarsError::ShapeMisMatch(_) => "Shape Mismatch",
        PolarsError::ComputeError(_) => "Computer error",
        PolarsError::OutOfBounds(_) => "Out Of Bounds",
        PolarsError::NoSlice => "No Slice",
        PolarsError::NoData(_) => "No Data",
        PolarsError::ValueError(_) => "Value Error",
        PolarsError::MemoryNotAligned => "Memory Not Aligned",
        PolarsError::RandError(_) => "Rand Error",
        PolarsError::HasNullValues(_) => "Has Null Values",
        PolarsError::UnknownSchema(_) => "Unknown Schema",
        PolarsError::Various(_) => "Various",
        PolarsError::Io(_) => "Io Error",
        PolarsError::Regex(_) => "Regex Error",
        PolarsError::Duplicate(_) => "Duplicate Error",
        PolarsError::ImplementationError => "Implementation Error",
    };

    let label = e.to_string();

    match secondary {
        None => ShellError::labeled_error(msg, label, span),
        Some(s) => ShellError::labeled_error_with_secondary(msg, label, span, s.as_ref(), span),
    }
}
