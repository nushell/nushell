use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::values::{CustomValueSupport, PolarsPluginCustomValue, PolarsPluginType};

use super::NuSchema;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NuSchemaCustomValue {
    pub id: Uuid,
    #[serde(skip)]
    pub datatype: Option<NuSchema>,
}

#[typetag::serde]
impl CustomValue for NuSchemaCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        PolarsPluginType::NuSchema.type_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuSchema: custom_value_to_base_value should've been called",
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

impl PolarsPluginCustomValue for NuSchemaCustomValue {
    type PolarsPluginObjectType = NuSchema;

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
        let dtype = NuSchema::try_from_custom_value(plugin, self)?;
        dtype.base_value(Span::unknown())
    }
}
