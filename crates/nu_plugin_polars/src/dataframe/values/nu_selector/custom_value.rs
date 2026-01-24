use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::values::{CustomValueSupport, PolarsPluginCustomValue, PolarsPluginType};

use super::NuSelector;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NuSelectorCustomValue {
    pub id: Uuid,
    #[serde(skip)]
    pub selector: Option<NuSelector>,
}

#[typetag::serde]
impl CustomValue for NuSelectorCustomValue {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        PolarsPluginType::NuSelector.type_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuSelectorValue: custom_value_to_base_value should've been called",
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

impl PolarsPluginCustomValue for NuSelectorCustomValue {
    type PolarsPluginObjectType = NuSelector;

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PolarsPluginObjectType> {
        &self.selector
    }

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        let selector = NuSelector::try_from_custom_value(plugin, self)?;
        selector.base_value(Span::unknown())
    }

    fn custom_value_operation(
        &self,
        plugin: &crate::PolarsPlugin,
        engine: &nu_plugin::EngineInterface,
        left_span: Span,
        operator: nu_protocol::Spanned<nu_protocol::ast::Operator>,
        right: Value,
    ) -> Result<Value, ShellError> {
        // Convert selector to expression, then perform operation
        let selector = NuSelector::try_from_custom_value(plugin, self)?;
        let expr = selector.into_expr();

        // Now perform the operation on the expression
        let expr_cv = expr.custom_value();
        expr_cv.custom_value_operation(plugin, engine, left_span, operator, right)
    }
}
