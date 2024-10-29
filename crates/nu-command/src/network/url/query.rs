use std::borrow::Cow;

use nu_protocol::{Record, ShellError, Span, Type, Value};

pub fn record_to_query_string(
    record: &Record,
    span: Span,
    head: Span,
) -> Result<String, ShellError> {
    let mut row_vec = vec![];
    for (k, v) in record {
        match v {
            Value::List { ref vals, .. } => {
                for v_item in vals {
                    row_vec.push((
                        k.as_str(),
                        v_item
                            .coerce_str()
                            .map_err(|_| ShellError::UnsupportedInput {
                                msg: "Expected a record with list of string values".to_string(),
                                input: "value originates from here".into(),
                                msg_span: head,
                                input_span: span,
                            })?,
                    ));
                }
            }
            _ => row_vec.push((
                k.as_str(),
                v.coerce_str().map_err(|_| ShellError::UnsupportedInput {
                    msg: "Expected a record with string or list of string values".to_string(),
                    input: "value originates from here".into(),
                    msg_span: head,
                    input_span: span,
                })?,
            )),
        }
    }

    serde_urlencoded::to_string(row_vec).map_err(|_| ShellError::CantConvert {
        to_type: "URL".into(),
        from_type: Type::record().to_string(),
        span: head,
        help: None,
    })
}

pub fn table_to_query_string(
    table: &[Value],
    span: Span,
    head: Span,
) -> Result<String, ShellError> {
    let row_vec = table
        .iter()
        .map(|val| match val {
            Value::Record { val, internal_span } => key_value_from_record(val, *internal_span),
            _ => Err(ShellError::UnsupportedInput {
                msg: "expected a table".into(),
                input: "not a table, contains non-record values".into(),
                msg_span: head,
                input_span: span,
            }),
        })
        .collect::<Result<Vec<_>, ShellError>>()?;

    serde_urlencoded::to_string(row_vec).map_err(|_| ShellError::CantConvert {
        to_type: "URL".into(),
        from_type: Type::table().to_string(),
        span: head,
        help: None,
    })
}

fn key_value_from_record(record: &Record, span: Span) -> Result<(Cow<str>, Cow<str>), ShellError> {
    let key = record
        .get("key")
        .ok_or_else(|| ShellError::CantFindColumn {
            col_name: "key".into(),
            span: None,
            src_span: span,
        })?
        .coerce_str()?;
    let value = record
        .get("value")
        .ok_or_else(|| ShellError::CantFindColumn {
            col_name: "value".into(),
            span: None,
            src_span: span,
        })?
        .coerce_str()?;
    Ok((key, value))
}
