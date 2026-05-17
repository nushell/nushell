use nu_protocol::{CustomValue, LabeledError, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

/// References a stored handle within the plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleCustomValue(pub u64);

impl HandleCustomValue {
    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

#[typetag::serde]
impl CustomValue for HandleCustomValue {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        "HandleCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Err(LabeledError::new("Unsupported operation")
            .with_label("can't call to_base_value() directly on this", span)
            .with_help("HandleCustomValue uses custom_value_to_base_value() on the plugin instead")
            .into())
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
