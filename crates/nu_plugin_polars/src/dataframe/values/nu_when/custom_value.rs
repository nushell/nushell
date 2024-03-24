use crate::{CustomValueSupport, PolarsPluginCustomValue};

use super::{NuWhen, NuWhenType};
use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NuWhenCustomValue {
    pub id: uuid::Uuid,
    #[serde(skip)]
    pub when: Option<NuWhen>,
}

// CustomValue implementation for NuWhen
#[typetag::serde]
impl CustomValue for NuWhenCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "NuWhenCustomValue".into()
    }

    fn to_base_value(&self, _span: Span) -> Result<Value, ShellError> {
        panic!("NuWhenCustomValue: custom_value_to_base_value should've been called")
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}

impl PolarsPluginCustomValue for NuWhenCustomValue {
    type PhysicalType = NuWhen;

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        let when = NuWhen::try_from_custom_value(plugin, self)?;
        let val: String = match when.when_type {
            NuWhenType::Then(_) => "whenthen".into(),
            NuWhenType::ChainedThen(_) => "whenthenthen".into(),
        };

        let value = Value::string(val, Span::unknown());
        Ok(value)
    }

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PhysicalType> {
        &self.when
    }
}
