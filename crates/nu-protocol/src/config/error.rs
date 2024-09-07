use super::ConfigPath;
use crate::{Config, ShellError, Span, Type, Value};

#[derive(Debug)]
pub(super) struct ConfigErrors<'a> {
    config: &'a Config,
    errors: Vec<ShellError>,
}

impl<'a> ConfigErrors<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            errors: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn error_raw(&mut self, error: ShellError) {
        self.errors.push(error);
    }

    pub fn error(&mut self, path: &ConfigPath, error: ShellError) {
        self.errors.push(ShellError::InvalidConfigValue {
            path: path.to_string(),
            error: vec![error],
        });
    }

    pub fn type_mismatch(&mut self, path: &ConfigPath, expected: Type, actual: &Value) {
        self.error(
            path,
            ShellError::RuntimeTypeMismatch {
                expected,
                actual: actual.get_type(),
                span: actual.span(),
            },
        );
    }

    pub fn incorrect_value(
        &mut self,
        path: &ConfigPath,
        expected: impl Into<String>,
        actual: &Value,
    ) {
        self.error(
            path,
            ShellError::InvalidValue {
                valid: expected.into(),
                actual: if let Ok(str) = actual.as_str() {
                    format!("'{str}'")
                } else {
                    actual.to_abbreviated_string(self.config)
                },
                span: actual.span(),
            },
        );
    }

    pub fn missing_value(&mut self, path: &ConfigPath, column: &'static str, span: Span) {
        self.error(path, ShellError::MissingColumn { column, span })
    }

    pub fn unknown_value(&mut self, path: &ConfigPath, value: &Value) {
        self.errors.push(ShellError::UnknownConfigValue {
            path: path.to_string(),
            span: value.span(),
        });
    }

    pub fn into_shell_error(self) -> Option<ShellError> {
        if self.is_empty() {
            None
        } else {
            Some(ShellError::InvalidConfig {
                errors: self.errors,
            })
        }
    }
}
