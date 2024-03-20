use super::NuWhen;
use nu_protocol::{CustomValue, ShellError, Span, Value};

// CustomValue implementation for NuDataFrame
#[typetag::serde]
impl CustomValue for NuWhen {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = self.clone();

        Value::custom_value(Box::new(cloned), span)
    }

    fn type_name(&self) -> String {
        "NuWhen".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let val: String = match self {
            NuWhen::Then(_) => "whenthen".into(),
            NuWhen::ChainedThen(_) => "whenthenthen".into(),
        };

        let value = Value::string(val, span);
        Ok(value)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}
