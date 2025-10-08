use std::cmp::Ordering;

use nu_plugin::EngineInterface;
use nu_protocol::{CustomValue, ShellError, Span, Value};
use polars::prelude::{col, nth};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    Cacheable, PolarsPlugin,
    values::{
        CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginCustomValue, PolarsPluginType,
    },
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
        PolarsPluginType::NuLazyFrame.type_name().to_string()
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
        let eager = NuLazyFrame::try_from_custom_value(plugin, self)?.collect(Span::unknown())?;
        let other = NuDataFrame::try_from_value_coerce(plugin, &other_value, other_value.span())?;
        let res = eager.is_equal(&other);
        Ok(res)
    }

    fn custom_value_operation(
        &self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        lhs_span: Span,
        operator: nu_protocol::Spanned<nu_protocol::ast::Operator>,
        right: Value,
    ) -> Result<Value, ShellError> {
        let eager = NuLazyFrame::try_from_custom_value(plugin, self)?.collect(Span::unknown())?;
        Ok(eager
            .compute_with_value(plugin, lhs_span, operator.item, operator.span, &right)?
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span))
    }

    fn custom_value_follow_path_int(
        &self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        _self_span: Span,
        index: nu_protocol::Spanned<usize>,
    ) -> Result<Value, ShellError> {
        let expr = NuExpression::from(nth(index.item as i64).as_expr());
        expr.cache_and_to_value(plugin, engine, index.span)
    }

    fn custom_value_follow_path_string(
        &self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        _self_span: Span,
        column_name: nu_protocol::Spanned<String>,
    ) -> Result<Value, ShellError> {
        let expr = NuExpression::from(col(column_name.item));
        expr.cache_and_to_value(plugin, engine, column_name.span)
    }
}
