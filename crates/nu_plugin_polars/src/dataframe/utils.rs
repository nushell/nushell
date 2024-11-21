use nu_protocol::{FromValue, ShellError, Value};
use polars::prelude::PlSmallStr;

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

pub fn extract_sm_strs(value: Value) -> Result<Vec<PlSmallStr>, ShellError> {
    let span = value.span();
    match (
        <String as FromValue>::from_value(value.clone()),
        <Vec<String> as FromValue>::from_value(value),
    ) {
        (Ok(col), Err(_)) => Ok(vec![col.into()]),
        (Err(_), Ok(cols)) => Ok(cols.iter().map(PlSmallStr::from).collect()),
        _ => Err(ShellError::IncompatibleParametersSingle {
            msg: "Expected a string or list of strings".into(),
            span,
        }),
    }
}
