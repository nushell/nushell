use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuLazyFrameCustomValue {
    pub id: Uuid,
    pub val: Value,
}

// CustomValue implementation for NuDataFrame
#[typetag::serde]
impl CustomValue for NuLazyFrameCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        "NuLazyFrameCustomValue".into()
    }

    fn to_base_value(&self, _span: Span) -> Result<Value, ShellError> {
        Ok(self.val.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
