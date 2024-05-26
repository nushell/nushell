use nu_protocol::{FromValue, ShellError, Span, Value};

pub fn extract_strings(value: Value) -> Result<Vec<String>, ShellError> {
    let span = value.span();
    match (
        // Both String and Vec<_> don't use the call_span in the FromValue impl.
        <String as FromValue>::from_value(value.clone(), Span::unknown()),
        <Vec<String> as FromValue>::from_value(value, Span::unknown()),
    ) {
        (Ok(col), Err(_)) => Ok(vec![col]),
        (Err(_), Ok(cols)) => Ok(cols),
        _ => Err(ShellError::IncompatibleParametersSingle {
            msg: "Expected a string or list of strings".into(),
            span,
        }),
    }
}
