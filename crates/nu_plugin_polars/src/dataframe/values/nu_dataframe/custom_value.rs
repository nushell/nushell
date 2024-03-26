use nu_plugin::EngineInterface;
use nu_protocol::{CustomValue, ShellError, Span, Spanned, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Cacheable, CustomValueSupport, PolarsPlugin, PolarsPluginCustomValue};

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
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "NuDataFrameCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuDataFrameValue: custom_value_to_base_value should've been called",
            span,
        ))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    // fn follow_path_string(
    //     &self,
    //     _self_span: Span,
    //     column_name: String,
    //     path_span: Span,
    // ) -> Result<Value, ShellError> {
    //     let df = NuDataFrame::try_from(self)?;
    //     let column = df.column(&column_name, path_span)?;
    //     Ok(column.into_value(path_span))
    // }
    //
    // fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
    //     if let Ok(df) = NuDataFrame::try_from(self) {
    //         if let Value::CustomValue { val, .. } = other {
    //             val.as_any()
    //                 .downcast_ref::<NuDataFrameCustomValue>()
    //                 .and_then(|other| NuDataFrame::try_from(other).ok())
    //                 .and_then(|ref other| df.is_equal(other))
    //         } else {
    //             None
    //         }
    //     } else {
    //         None
    //     }
    // }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}

impl PolarsPluginCustomValue for NuDataFrameCustomValue {
    type PhysicalType = NuDataFrame;

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PhysicalType> {
        &self.dataframe
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
            .cache(plugin, engine)?
            .into_value(lhs_span))
    }

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from_custom_value(plugin, self)?;
        df.base_value(Span::unknown())
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
}
