use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
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
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn try_from_value(value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(value) => Ok(value.clone()),
                None => Err(ShellError::CantConvert {
                    to_type: "cool".into(),
                    from_type: "non-cool".into(),
                    span: *span,
                    help: None,
                }),
            },
            x => Err(ShellError::CantConvert {
                to_type: "cool".into(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }
}

#[typetag::serde]
impl CustomValue for SecondCustomValue {
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
            val: format!(
                "I used to be a DIFFERENT custom value! ({})",
                self.something
            ),
            span,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
