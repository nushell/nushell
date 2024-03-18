use crate::DataFrameCache;

use super::NuLazyGroupBy;
use nu_protocol::{record, CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuLazyGroupByCustomValue {
    pub id: Uuid,
}

impl TryFrom<&NuLazyGroupByCustomValue> for NuLazyGroupBy {
    type Error = ShellError;

    fn try_from(value: &NuLazyGroupByCustomValue) -> Result<Self, Self::Error> {
        DataFrameCache::instance()
            .get_group_by(&value.id)
            .ok_or_else(|| ShellError::GenericError {
                error: format!("GroupBy {:?} not found in cache", value.id),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
    }
}

#[typetag::serde]
impl CustomValue for NuLazyGroupByCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        Value::custom_value(Box::new(self.clone()), span)
    }

    fn value_string(&self) -> String {
        "NuLazyGroupByCustomValue".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::record(
            record! {
                "LazyGroupBy" => Value::string("apply aggregation to complete execution plan", span)
            },
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
