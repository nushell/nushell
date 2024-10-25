use nu_protocol::{Record, ShellError, Span, Type, Value};

pub fn record_to_qs(record: &Record, span: Span, head: Span) -> Result<String, ShellError> {
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
                v.coerce_str()
                    .map_err(|_| ShellError::UnsupportedInput {
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
