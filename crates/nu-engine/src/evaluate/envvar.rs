use std::convert::TryFrom;

use nu_errors::ShellError;
use nu_protocol::Value;

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
        } else {
            Ok(EnvVar::Proper(value.as_string()?))
        }
    }
}

impl TryFrom<&Value> for EnvVar {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        if value.value.is_none() {
            Ok(EnvVar::Nothing)
        } else {
            Ok(EnvVar::Proper(value.as_string()?))
        }
    }
}

impl From<String> for EnvVar {
    fn from(string: String) -> Self {
        EnvVar::Proper(string)
    }
}
