use std::cmp::Ordering;

use nu_plugin::EngineInterface;
use nu_protocol::{CustomValue, ShellError, Span, Spanned, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Cacheable, PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginCustomValue, PolarsPluginType},
};

use super::NuDataFrame;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NuDataFrameCustomValue {
    pub id: Uuid,
    #[serde(skip)]
    pub dataframe: Option<NuDataFrame>,
}

// CustomValue implementation for NuDataFrame
#[typetag::serde]
impl CustomValue for NuDataFrameCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        PolarsPluginType::NuDataFrame.type_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuDataFrameValue: custom_value_to_base_value should've been called",
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

impl PolarsPluginCustomValue for NuDataFrameCustomValue {
    type PolarsPluginObjectType = NuDataFrame;

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PolarsPluginObjectType> {
        &self.dataframe
    }

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from_custom_value(plugin, self)?;
        df.base_value(Span::unknown())
    }

    fn custom_value_operation(
        &self,
        plugin: &crate::PolarsPlugin,
        engine: &nu_plugin::EngineInterface,
        lhs_span: Span,
        operator: nu_protocol::Spanned<nu_protocol::ast::Operator>,
        right: Value,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from_custom_value(plugin, self)?;
        Ok(df
            .compute_with_value(plugin, lhs_span, operator.item, operator.span, &right)?
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span))
    }

    fn custom_value_follow_path_int(
        &self,
        plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        _self_span: Span,
        index: Spanned<usize>,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from_custom_value(plugin, self)?;
        df.get_value(index.item, index.span)
    }

    fn custom_value_follow_path_string(
        &self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        self_span: Span,
        column_name: Spanned<String>,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from_custom_value(plugin, self)?;
        let column = df.column(&column_name.item, self_span)?;
        Ok(column
            .cache(plugin, engine, self_span)?
            .into_value(self_span))
    }

    fn custom_value_partial_cmp(
        &self,
        plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        other_value: Value,
    ) -> Result<Option<Ordering>, ShellError> {
        let df = NuDataFrame::try_from_custom_value(plugin, self)?;
        let other = NuDataFrame::try_from_value_coerce(plugin, &other_value, other_value.span())?;
        let res = df.is_equal(&other);
        Ok(res)
    }
}
