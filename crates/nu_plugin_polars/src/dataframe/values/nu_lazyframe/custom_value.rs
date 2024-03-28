use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CustomValueSupport, PolarsPluginCustomValue};

use super::NuLazyFrame;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuLazyFrameCustomValue {
    pub id: Uuid,
    #[serde(skip)]
    pub lazyframe: Option<NuLazyFrame>,
}

// CustomValue implementation for NuDataFrame
#[typetag::serde]
impl CustomValue for NuLazyFrameCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "NuLazyFrameCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuLazyFrameCustomValue: custom_value_to_base_value should've been called",
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

impl PolarsPluginCustomValue for NuLazyFrameCustomValue {
    type PolarsPluginObjectType = NuLazyFrame;

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        let lazy = NuLazyFrame::try_from_custom_value(plugin, self)?;
        lazy.base_value(Span::unknown())
    }

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PolarsPluginObjectType> {
        &self.lazyframe
    }
}
