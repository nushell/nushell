use crate::{Record, ShellError, Span, Value};
use std::{collections::HashMap, fmt::Display, str::FromStr};

pub(super) trait ReconstructVal {
    fn reconstruct_value(&self, span: Span) -> Value;
}

pub(super) fn process_string_enum<T, E>(
    config_point: &mut T,
    config_path: &[&str],
    value: &mut Value,
    errors: &mut Vec<ShellError>,
) where
    T: FromStr<Err = E> + ReconstructVal,
    E: Display,
{
    let span = value.span();
    if let Ok(v) = value.coerce_str() {
        match v.parse() {
            Ok(format) => {
                *config_point = format;
            }
            Err(err) => {
                errors.push(ShellError::GenericError {
                    error: "Error while applying config changes".into(),
                    msg: format!(
                        "unrecognized $env.config.{} option '{v}'",
                        config_path.join(".")
                    ),
                    span: Some(span),
                    help: Some(err.to_string()),
                    inner: vec![],
                });
                // Reconstruct
                *value = config_point.reconstruct_value(span);
            }
        }
    } else {
        errors.push(ShellError::GenericError {
            error: "Error while applying config changes".into(),
            msg: format!("$env.config.{} should be a string", config_path.join(".")),
            span: Some(span),
            help: Some("This value will be ignored.".into()),
            inner: vec![],
        });
        // Reconstruct
        *value = config_point.reconstruct_value(span);
    }
}

pub(super) fn process_bool_config(
    value: &mut Value,
    errors: &mut Vec<ShellError>,
    config_point: &mut bool,
) {
    if let Ok(b) = value.as_bool() {
        *config_point = b;
    } else {
        errors.push(ShellError::GenericError {
            error: "Error while applying config changes".into(),
            msg: "should be a bool".to_string(),
            span: Some(value.span()),
            help: Some("This value will be ignored.".into()),
            inner: vec![],
        });
        // Reconstruct
        *value = Value::bool(*config_point, value.span());
    }
}

pub(super) fn process_int_config(
    value: &mut Value,
    errors: &mut Vec<ShellError>,
    config_point: &mut i64,
) {
    if let Ok(b) = value.as_int() {
        *config_point = b;
    } else {
        errors.push(ShellError::GenericError {
            error: "Error while applying config changes".into(),
            msg: "should be an int".into(),
            span: Some(value.span()),
            help: Some("This value will be ignored.".into()),
            inner: vec![],
        });
        // Reconstruct
        *value = Value::int(*config_point, value.span());
    }
}

pub(super) fn report_invalid_key(keys: &[&str], span: Span, errors: &mut Vec<ShellError>) {
    // Because Value::Record discards all of the spans of its
    // column names (by storing them as Strings), the key name cannot be provided
    // as a value, even in key errors.
    errors.push(ShellError::GenericError {
        error: "Error while applying config changes".into(),
        msg: format!(
            "$env.config.{} is an unknown config setting",
            keys.join(".")
        ),
        span: Some(span),
        help: Some("This value will not appear in your $env.config record.".into()),
        inner: vec![],
    });
}

pub(super) fn report_invalid_value(msg: &str, span: Span, errors: &mut Vec<ShellError>) {
    errors.push(ShellError::GenericError {
        error: "Error while applying config changes".into(),
        msg: msg.into(),
        span: Some(span),
        help: Some("This value will be ignored.".into()),
        inner: vec![],
    });
}

pub(super) fn create_map(value: &Value) -> Result<HashMap<String, Value>, ShellError> {
    Ok(value
        .as_record()?
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect())
}

pub fn extract_value<'record>(
    name: &str,
    record: &'record Record,
    span: Span,
) -> Result<&'record Value, ShellError> {
    record
        .get(name)
        .ok_or_else(|| ShellError::MissingConfigValue {
            missing_value: name.to_string(),
            span,
        })
}
