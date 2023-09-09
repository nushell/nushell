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
        Value::custom_value(Box::new(self), span)
    }

    pub fn try_from_value(value: &Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::CustomValue { val, .. } => {
                if let Some(cool) = val.as_any().downcast_ref::<Self>() {
                    Ok(cool.clone())
                } else {
                    Err(ShellError::CantConvert {
                        to_type: "cool".into(),
                        from_type: "non-cool".into(),
                        span,
                        help: None,
                    })
                }
            }
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
impl CustomValue for CoolCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: nu_protocol::Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            format!("I used to be a custom value! My data was ({})", self.cool),
            span,
        ))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
