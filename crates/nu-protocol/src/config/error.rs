use super::ConfigPath;
use crate::{Config, ConfigError, ConfigWarning, ShellError, ShellWarning, Span, Type, Value};

#[derive(Debug)]
#[must_use]
pub(super) struct ConfigErrors<'a> {
    config: &'a Config,
    errors: Vec<ConfigError>,
    warnings: Vec<ConfigWarning>,
}

impl<'a> ConfigErrors<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            config,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn error(&mut self, error: ConfigError) {
        self.errors.push(error);
    }

    pub fn warn(&mut self, warning: ConfigWarning) {
        self.warnings.push(warning);
    }

    pub fn type_mismatch(&mut self, path: &ConfigPath, expected: Type, actual: &Value) {
        self.error(ConfigError::TypeMismatch {
            path: path.to_string(),
            expected,
            actual: actual.get_type(),
            span: actual.span(),
        });
    }

    pub fn invalid_value(
        &mut self,
        path: &ConfigPath,
        expected: impl Into<String>,
        actual: &Value,
    ) {
        self.error(ConfigError::InvalidValue {
            path: path.to_string(),
            valid: expected.into(),
            actual: if let Ok(str) = actual.as_str() {
                format!("'{str}'")
            } else {
                actual.to_abbreviated_string(self.config)
            },
            span: actual.span(),
        });
    }

    pub fn missing_column(&mut self, path: &ConfigPath, column: &'static str, span: Span) {
        self.error(ConfigError::MissingRequiredColumn {
            path: path.to_string(),
            column,
            span,
        })
    }

    pub fn unknown_option(&mut self, path: &ConfigPath, value: &Value) {
        self.error(ConfigError::UnknownOption {
            path: path.to_string(),
            span: value.span(),
        });
    }

    // We'll probably need this again in the future so allow dead code for now
    #[allow(dead_code)]
    pub fn deprecated_option(&mut self, path: &ConfigPath, suggestion: &'static str, span: Span) {
        self.error(ConfigError::Deprecated {
            path: path.to_string(),
            suggestion,
            span,
        });
    }

    pub fn check(self) -> Result<Option<ShellWarning>, ShellError> {
        match (self.has_errors(), self.has_warnings()) {
            (true, _) => Err(ShellError::InvalidConfig {
                errors: self.errors,
            }),
            (false, true) => Ok(Some(ShellWarning::InvalidConfig {
                warnings: self.warnings,
            })),
            (false, false) => Ok(None),
        }
    }
}
