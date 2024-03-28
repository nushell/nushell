use crate::{CustomValueSupport, PolarsPluginCustomValue};

use super::NuLazyGroupBy;
use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuLazyGroupByCustomValue {
    pub id: Uuid,
    pub groupby: Option<NuLazyGroupBy>,
}

#[typetag::serde]
impl CustomValue for NuLazyGroupByCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "NuLazyGroupByCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuLazyGroupByCustomValue: custom_value_to_base_value should've been called",
            span,
        ))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}

impl PolarsPluginCustomValue for NuLazyGroupByCustomValue {
    type PhysicalType = NuLazyGroupBy;

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        NuLazyGroupBy::try_from_custom_value(plugin, self)?.base_value(Span::unknown())
    }

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PhysicalType> {
        &self.groupby
    }
}
