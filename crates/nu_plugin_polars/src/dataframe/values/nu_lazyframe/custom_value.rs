use std::cmp::Ordering;

use nu_plugin::EngineInterface;
use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    values::{CustomValueSupport, NuDataFrame, PolarsPluginCustomValue},
    PolarsPlugin,
};

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

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
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

    fn custom_value_partial_cmp(
        &self,
        plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        other_value: Value,
    ) -> Result<Option<Ordering>, ShellError> {
        // to compare, we need to convert to NuDataframe
        let df = NuLazyFrame::try_from_custom_value(plugin, self)?;
        let df = df.collect(other_value.span())?;
        let other = NuDataFrame::try_from_value_coerce(plugin, &other_value, other_value.span())?;
        let res = df.is_equal(&other);
        Ok(res)
    }
}
