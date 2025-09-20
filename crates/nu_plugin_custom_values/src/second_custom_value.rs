#![allow(clippy::result_large_err)]
use nu_protocol::{CustomValue, ShellError, Span, Spanned, Value, shell_error::io::IoError};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecondCustomValue {
    pub(crate) something: String,
}

impl SecondCustomValue {
    pub fn new(content: &str) -> Self {
        Self {
            something: content.to_owned(),
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    pub fn try_from_value(value: &Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::Custom { val, .. } => match val.as_any().downcast_ref::<Self>() {
                Some(value) => Ok(value.clone()),
                None => Err(ShellError::CantConvert {
                    to_type: "cool".into(),
                    from_type: "non-cool".into(),
                    span,
                    help: None,
                }),
            },
            x => Err(ShellError::CantConvert {
                to_type: "cool".into(),
                from_type: x.get_type().to_string(),
                span,
                help: None,
            }),
        }
    }
}

#[typetag::serde]
impl CustomValue for SecondCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: nu_protocol::Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            format!(
                "I used to be a DIFFERENT custom value! ({})",
                self.something
            ),
            span,
        ))
    }

    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        if let Value::Custom { val, .. } = other {
            val.as_any()
                .downcast_ref()
                .and_then(|other: &SecondCustomValue| PartialOrd::partial_cmp(self, other))
        } else {
            None
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn save(&self, path: Spanned<&Path>, _: Span, save_span: Span) -> Result<(), ShellError> {
        std::fs::write(path.item, &self.something).map_err(|err| {
            ShellError::Io(IoError::new_with_additional_context(
                err,
                save_span,
                path.item.to_owned(),
                format!("Could not save {}", self.type_name()),
            ))
        })
    }
}
