use nu_protocol::{CustomValue, Value};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct PluginCustomValue {
    pub data: serde_json::Value,
}

impl CustomValue for PluginCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> nu_protocol::Value {
        Value::CustomValue {
            val: Box::new(self.clone()),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(
        &self,
        _span: nu_protocol::Span,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        todo!()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn typetag_name(&self) -> &'static str {
        // TODO: Is this a good idea? I'd love to be able to get the name of the data type itself
        // but I don't think there's a way to get a &'static str without leaking which isn't ideal
        "PluginCustomValue"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}
