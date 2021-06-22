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
    let (msg, label) = match e {
        PolarsError::PolarsArrowError(_) => ("PolarsArrow Error", format!("{}", e)),
        PolarsError::ArrowError(_) => ("Arrow Error", format!("{}", e)),
        PolarsError::InvalidOperation(_) => ("Invalid Operation", format!("{}", e)),
        PolarsError::DataTypeMisMatch(_) => ("Data Type Mismatch", format!("{}", e)),
        PolarsError::NotFound(_) => ("Not Found", format!("{}", e)),
        PolarsError::ShapeMisMatch(_) => ("Shape Mismatch", format!("{}", e)),
        PolarsError::Other(_) => ("Other", format!("{}", e)),
        PolarsError::OutOfBounds(_) => ("Out Of Bounds", format!("{}", e)),
        PolarsError::NoSlice => ("No Slice", format!("{}", e)),
        PolarsError::NoData(_) => ("No Data", format!("{}", e)),
        PolarsError::ValueError(_) => ("Value Error", format!("{}", e)),
        PolarsError::MemoryNotAligned => ("Memory Not Aligned", format!("{}", e)),
        PolarsError::ParquetError(_) => ("Parquet Error", format!("{}", e)),
        PolarsError::RandError(_) => ("Rand Error", format!("{}", e)),
        PolarsError::HasNullValues(_) => ("Has Null Values", format!("{}", e)),
        PolarsError::UnknownSchema(_) => ("Unknown Schema", format!("{}", e)),
        PolarsError::Various(_) => ("Various", format!("{}", e)),
        PolarsError::Io(_) => ("Io Error", format!("{}", e)),
        PolarsError::Regex(_) => ("Regex Error", format!("{}", e)),
        PolarsError::Duplicate(_) => ("Duplicate Error", format!("{}", e)),
        PolarsError::ImplementationError => ("Implementation Error", format!("{}", e)),
    };

    match secondary {
        None => ShellError::labeled_error(msg, label, span),
        Some(s) => ShellError::labeled_error_with_secondary(msg, label, span, s.as_ref(), span),
    }
}
