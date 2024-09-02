use crate::{IntoValue, Record, ShellError, Span, Value};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    ops::{Deref, DerefMut},
    str::FromStr,
};

pub(super) struct ConfigPath<'a> {
    components: Vec<&'a str>,
}

impl<'a> ConfigPath<'a> {
    pub fn new() -> Self {
        Self {
            components: vec!["$env.config"],
        }
    }

    pub fn push(&mut self, key: &'a str) -> ConfigPathScope<'_, 'a> {
        self.components.push(key);
        ConfigPathScope { inner: self }
    }
}

impl Display for ConfigPath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.components.join("."))
    }
}

pub(super) struct ConfigPathScope<'whole, 'part> {
    inner: &'whole mut ConfigPath<'part>,
}

impl Drop for ConfigPathScope<'_, '_> {
    fn drop(&mut self) {
        self.inner.components.pop();
    }
}

impl<'a> Deref for ConfigPathScope<'_, 'a> {
    type Target = ConfigPath<'a>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl DerefMut for ConfigPathScope<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

pub(super) trait UpdateFromValue: Sized {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut Vec<ShellError>,
    );
}

impl UpdateFromValue for bool {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
        if let Ok(val) = value.as_bool() {
            *self = val;
        } else {
            report_invalid_config_value("should be a bool", value.span(), path, errors);
        }
    }
}

impl UpdateFromValue for i64 {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
        if let Ok(val) = value.as_int() {
            *self = val;
        } else {
            report_invalid_config_value("should be an int", value.span(), path, errors);
        }
    }
}

impl UpdateFromValue for usize {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
        if let Some(val) = value.as_int().ok().and_then(|v| v.try_into().ok()) {
            *self = val;
        } else {
            report_invalid_config_value(
                "should be a non-negative integer",
                value.span(),
                path,
                errors,
            );
        }
    }
}

impl UpdateFromValue for String {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
        if let Ok(val) = value.as_str() {
            *self = val.into();
        } else {
            report_invalid_config_value("should be a string", value.span(), path, errors);
        }
    }
}

impl UpdateFromValue for HashMap<String, Value> {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut Vec<ShellError>) {
        if let Ok(record) = value.as_record() {
            self.clear();
            self.extend(record.iter().map(|(k, v)| (k.clone(), v.clone())));
        } else {
            report_invalid_config_value("should be a record", value.span(), path, errors);
        }
    }
}

pub(super) fn config_update_string_enum<T>(
    choice: &mut T,
    value: &Value,
    path: &mut ConfigPath,
    errors: &mut Vec<ShellError>,
) where
    T: FromStr,
    T::Err: Display,
{
    let span = value.span();
    if let Ok(str) = value.as_str() {
        match str.parse() {
            Ok(val) => *choice = val,
            Err(err) => {
                errors.push(ShellError::GenericError {
                    error: "Error while applying config changes".into(),
                    msg: format!("unrecognized option for {path}"),
                    span: Some(span),
                    help: Some(err.to_string()),
                    inner: vec![],
                });
            }
        }
    } else {
        report_invalid_config_value("should be a string", span, path, errors);
    }
}

pub(super) fn process_string_enum<T, E>(
    config_point: &mut T,
    config_path: &[&str],
    value: &mut Value,
    errors: &mut Vec<ShellError>,
) where
    T: FromStr<Err = E> + Copy + IntoValue,
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
                *value = config_point.into_value(span);
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
        *value = config_point.into_value(span);
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

pub(super) fn report_invalid_config_value(
    msg: &str,
    span: Span,
    path: &ConfigPath,
    errors: &mut Vec<ShellError>,
) {
    errors.push(ShellError::GenericError {
        error: format!("Error while applying config changes to {path}"),
        msg: msg.into(),
        span: Some(span),
        help: Some("this value will be ignored.".into()),
        inner: vec![],
    });
}

pub(super) fn report_invalid_config_key(
    span: Span,
    path: &ConfigPath,
    errors: &mut Vec<ShellError>,
) {
    // Because Value::Record discards all of the spans of its
    // column names (by storing them as Strings), the key name cannot be provided
    // as a value, even in key errors.
    errors.push(ShellError::GenericError {
        error: "Error while applying config changes".into(),
        msg: format!("{path} is an unknown config setting"),
        span: Some(span),
        help: Some("this value will be ignored.".into()),
        inner: vec![],
    });
}

pub(super) fn report_missing_config_key(
    key: &str,
    span: Span,
    path: &ConfigPath,
    errors: &mut Vec<ShellError>,
) {
    errors.push(ShellError::GenericError {
        error: format!("Error while applying config changes to {path}",),
        msg: format!("{key} was not provided"),
        span: Some(span),
        help: Some("Please consult the documentation for configuring Nushell.".into()),
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
