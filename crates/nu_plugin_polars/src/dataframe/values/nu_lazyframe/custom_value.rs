use nu_protocol::{CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::DataFrameCache;

use super::NuLazyFrame;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuLazyFrameCustomValue {
    pub id: Uuid,
}

impl TryFrom<&NuLazyFrameCustomValue> for NuLazyFrame {
    type Error = ShellError;
    fn try_from(value: &NuLazyFrameCustomValue) -> Result<Self, Self::Error> {
        DataFrameCache::instance()
            .get_lazy(&value.id)
            .ok_or_else(|| ShellError::GenericError {
                error: format!("LazyFrame {:?} not found in cache", value.id),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
    }
}

// CustomValue implementation for NuDataFrame
#[typetag::serde]
impl CustomValue for NuLazyFrameCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        "NuLazyFrameCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let lazy = NuLazyFrame::try_from(self)?;
        lazy.base_value(span)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}
