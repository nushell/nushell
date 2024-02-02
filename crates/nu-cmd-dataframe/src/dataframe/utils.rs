use nu_protocol::{FromValue, ShellError, Value};

pub fn extract_strings(value: Value) -> Result<Vec<String>, ShellError> {
    let span = value.span();
    match (
        <String as FromValue>::from_value(value.clone()),
        <Vec<String> as FromValue>::from_value(value),
    ) {
        (Ok(col), Err(_)) => Ok(vec![col]),
        (Err(_), Ok(cols)) => Ok(cols),
        _ => Err(ShellError::IncompatibleParametersSingle {
            msg: "Expected a string or list of strings".into(),
            span,
        }),
    }
}
