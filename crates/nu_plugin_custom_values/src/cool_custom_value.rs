use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoolCustomValue {
    pub(crate) cool: String,
}

impl CoolCustomValue {
    pub fn new(content: &str) -> Self {
        Self {
            cool: content.to_owned(),
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn try_from_value(value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(cool) => Ok(cool.clone()),
                None => Err(ShellError::CantConvert(
                    "cool".into(),
                    "non-cool".into(),
                    *span,
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "cool".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }
}

#[typetag::serde]
impl CustomValue for CoolCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::CustomValue {
            val: Box::new(self.clone()),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: nu_protocol::Span) -> Result<Value, ShellError> {
        Ok(Value::String {
            val: format!("I used to be a custom value! My data was ({})", self.cool),
            span,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
