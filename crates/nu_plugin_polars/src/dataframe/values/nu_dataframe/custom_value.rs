use nu_protocol::{ast::Operator, CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DataFrameCache;

use super::NuDataFrame;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NuDataFrameCustomValue {
    pub id: Uuid,
    #[serde(skip)]
    pub dataframe: Option<NuDataFrame>,
}

impl TryFrom<&NuDataFrameCustomValue> for NuDataFrame {
    type Error = ShellError;

    fn try_from(value: &NuDataFrameCustomValue) -> Result<Self, Self::Error> {
        if let Some(df) = &value.dataframe {
            Ok(df.clone())
        } else {
            DataFrameCache::get_df(&value.id)?.ok_or_else(|| ShellError::GenericError {
                error: format!("Dataframe {:?} not found in cache", value.id),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    }
}

impl From<NuDataFrame> for NuDataFrameCustomValue {
    fn from(df: NuDataFrame) -> Self {
        Self {
            id: df.id,
            dataframe: Some(df),
        }
    }
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
        let df = NuDataFrame::try_from(self)?;
        df.base_value(span)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(
        &self,
        _self_span: Span,
        count: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from(self)?;
        df.get_value(count, path_span)
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from(self)?;
        let column = df.column(&column_name, path_span)?;
        Ok(column.into_value(path_span))
    }

    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        if let Ok(df) = NuDataFrame::try_from(self) {
            if let Value::CustomValue { val, .. } = other {
                val.as_any()
                    .downcast_ref::<NuDataFrameCustomValue>()
                    .and_then(|other| NuDataFrame::try_from(other).ok())
                    .and_then(|ref other| df.is_equal(other))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        let df = NuDataFrame::try_from(self)?;
        df.compute_with_value(lhs_span, operator, op, right)
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}
