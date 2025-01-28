use super::error::ConfigErrors;
use crate::{Record, ShellError, Span, Type, Value};
use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{self, Display},
    hash::Hash,
    ops::{Deref, DerefMut},
    str::FromStr,
};

enum ConfigPathComponent<'a> {
    Col(&'a str),
    Row(usize),
}

pub(crate) struct ConfigPath<'a> {
    components: Vec<ConfigPathComponent<'a>>,
}

impl<'a> ConfigPath<'a> {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn push(&mut self, col: &'a str) -> ConfigPathScope<'_, 'a> {
        self.components.push(ConfigPathComponent::Col(col));
        ConfigPathScope { inner: self }
    }

    pub fn push_row(&mut self, row: usize) -> ConfigPathScope<'_, 'a> {
        self.components.push(ConfigPathComponent::Row(row));
        ConfigPathScope { inner: self }
    }
}

impl Display for ConfigPath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "$env.config")?;
        for component in &self.components {
            match component {
                ConfigPathComponent::Col(col) => write!(f, ".{col}"),
                ConfigPathComponent::Row(row) => write!(f, ".{row}"),
            }?
        }
        Ok(())
    }
}

pub(crate) struct ConfigPathScope<'whole, 'part> {
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

pub(crate) trait UpdateFromValue: Sized {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    );
}

impl UpdateFromValue for Value {
    fn update(&mut self, value: &Value, _path: &mut ConfigPath, _errors: &mut ConfigErrors) {
        *self = value.clone();
    }
}

impl UpdateFromValue for bool {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        if let Ok(val) = value.as_bool() {
            *self = val;
        } else {
            errors.type_mismatch(path, Type::Bool, value);
        }
    }
}

impl UpdateFromValue for i64 {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        if let Ok(val) = value.as_int() {
            *self = val;
        } else {
            errors.type_mismatch(path, Type::Int, value);
        }
    }
}

impl UpdateFromValue for usize {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        if let Ok(val) = value.as_int() {
            if let Ok(val) = val.try_into() {
                *self = val;
            } else {
                errors.invalid_value(path, "a non-negative integer", value);
            }
        } else {
            errors.type_mismatch(path, Type::Int, value);
        }
    }
}

impl UpdateFromValue for String {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        if let Ok(val) = value.as_str() {
            *self = val.into();
        } else {
            errors.type_mismatch(path, Type::String, value);
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
        errors: &mut ConfigErrors,
    ) {
        if let Ok(record) = value.as_record() {
            *self = record
                .iter()
                .map(|(key, val)| {
                    let mut old = self.remove(key).unwrap_or_default();
                    old.update(val, &mut path.push(key), errors);
                    (key.as_str().into(), old)
                })
                .collect();
        } else {
            errors.type_mismatch(path, Type::record(), value);
        }
    }
}

pub(super) fn config_update_string_enum<T>(
    choice: &mut T,
    value: &Value,
    path: &mut ConfigPath,
    errors: &mut ConfigErrors,
) where
    T: FromStr,
    T::Err: Display,
{
    if let Ok(str) = value.as_str() {
        match str.parse() {
            Ok(val) => *choice = val,
            Err(err) => errors.invalid_value(path, err.to_string(), value),
        }
    } else {
        errors.type_mismatch(path, Type::String, value);
    }
}

pub fn extract_value<'record>(
    column: &'static str,
    record: &'record Record,
    span: Span,
) -> Result<&'record Value, ShellError> {
    record
        .get(column)
        .ok_or_else(|| ShellError::MissingRequiredColumn { column, span })
}
