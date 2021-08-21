use std::convert::TryFrom;

use nu_errors::ShellError;
use nu_protocol::{SpannedTypeName, Value};

#[derive(Debug, Clone)]
pub enum EnvVar {
    Proper(String),
    Nothing,
}

impl TryFrom<Value> for EnvVar {
    type Error = ShellError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value.value.is_none() {
            Ok(EnvVar::Nothing)
        } else if value.is_primitive() {
            Ok(EnvVar::Proper(value.convert_to_string()))
        } else {
            Err(ShellError::type_error(
                "primitive",
                value.spanned_type_name(),
            ))
        }
    }
}

impl TryFrom<&Value> for EnvVar {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        if value.value.is_none() {
            Ok(EnvVar::Nothing)
        } else if value.is_primitive() {
            Ok(EnvVar::Proper(value.convert_to_string()))
        } else {
            Err(ShellError::type_error(
                "primitive",
                value.spanned_type_name(),
            ))
        }
    }
}

impl From<String> for EnvVar {
    fn from(string: String) -> Self {
        EnvVar::Proper(string)
    }
}
