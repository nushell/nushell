use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::values::{CustomValueSupport, PolarsPluginCustomValue, PolarsPluginType};

use super::NuDataType;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NuDataTypeCustomValue {
    pub id: Uuid,
    #[serde(skip)]
    pub datatype: Option<NuDataType>,
}

#[typetag::serde]
impl CustomValue for NuDataTypeCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        PolarsPluginType::NuDataType.type_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuDataType: custom_value_to_base_value should've been called",
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

impl PolarsPluginCustomValue for NuDataTypeCustomValue {
    type PolarsPluginObjectType = NuDataType;

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PolarsPluginObjectType> {
        &self.datatype
    }

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        let dtype = NuDataType::try_from_custom_value(plugin, self)?;
        dtype.base_value(Span::unknown())
    }
}
