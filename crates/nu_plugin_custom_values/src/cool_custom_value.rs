use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CoolCustomValue {
    cool: String,
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
}

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

    fn typetag_name(&self) -> &'static str {
        "FirestoreDatabase"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}
