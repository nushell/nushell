use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginData {
    data: Vec<u8>,
}

impl PluginData {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn can_downcast(value: &Value) -> bool {
        if let Value::CustomValue { val, .. } = value {
            val.as_any().downcast_ref::<Self>().is_some()
        } else {
            false
        }
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(data) => Ok(Self {
                    data: data.data.clone(),
                }),
                None => Err(ShellError::CantConvert(
                    "pipeline data".into(),
                    "custom value".into(),
                    span,
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "pipeline data".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }
}

impl CustomValue for PluginData {
    fn typetag_name(&self) -> &'static str {
        "plugin data"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: Span) -> Value {
        let cloned = Self {
            data: self.data.clone(),
        };

        Value::CustomValue {
            val: Box::new(cloned),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::String {
            val: "plugin data".into(),
            span,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
