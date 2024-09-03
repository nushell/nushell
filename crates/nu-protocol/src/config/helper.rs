use crate::{Record, ShellError, Span, Value};
use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{self, Display},
    hash::Hash,
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

impl UpdateFromValue for Value {
    fn update(&mut self, value: &Value, _path: &mut ConfigPath, _errors: &mut Vec<ShellError>) {
        *self = value.clone();
    }
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

impl<K, V> UpdateFromValue for HashMap<K, V>
where
    K: Borrow<str> + for<'a> From<&'a str> + Eq + Hash,
    V: Default + UpdateFromValue,
{
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut Vec<ShellError>,
    ) {
        if let Ok(record) = value.as_record() {
            self.retain(|k, _| record.contains(k.borrow()));
            for (key, val) in record {
                self.entry(key.as_str().into())
                    .or_default()
                    .update(val, path, errors);
            }
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
