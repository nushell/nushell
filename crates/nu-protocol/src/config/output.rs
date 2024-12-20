use std::io::IsTerminal;

use super::{config_update_string_enum, prelude::*};
use crate::{self as nu_protocol, FromValue};

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorStyle {
    Plain,
    Fancy,
}

impl FromStr for ErrorStyle {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "fancy" => Ok(Self::Fancy),
            "plain" => Ok(Self::Plain),
            _ => Err("'fancy' or 'plain'"),
        }
    }
}

impl UpdateFromValue for ErrorStyle {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, IntoValue, Serialize, Deserialize)]
pub enum UseAnsiColoring {
    #[default]
    Auto,
    True,
    False,
}

impl UseAnsiColoring {
    pub fn get(self) -> bool {
        match self {
            Self::Auto => std::io::stdout().is_terminal(),
            Self::True => true,
            Self::False => false,
        }
    }
}

impl FromValue for UseAnsiColoring {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        if let Ok(v) = v.as_bool() {
            return Ok(match v {
                true => Self::True,
                false => Self::False,
            });
        }

        #[derive(FromValue)]
        enum UseAnsiColoringString {
            Auto,
            True,
            False,
        }

        Ok(match UseAnsiColoringString::from_value(v)? {
            UseAnsiColoringString::Auto => Self::Auto,
            UseAnsiColoringString::True => Self::True,
            UseAnsiColoringString::False => Self::False,
        })
    }
}

impl UpdateFromValue for UseAnsiColoring {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        let Ok(value) = UseAnsiColoring::from_value(value.clone()) else {
            errors.type_mismatch(path, UseAnsiColoring::expected_type(), value);
            return;
        };

        *self = value;
    }
}
