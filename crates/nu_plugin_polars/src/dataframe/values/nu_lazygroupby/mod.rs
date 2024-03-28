mod custom_value;

use core::fmt;
use nu_protocol::{record, ShellError, Span, Value};
use polars::prelude::LazyGroupBy;
use std::sync::Arc;
use uuid::Uuid;

use crate::Cacheable;

pub use self::custom_value::NuLazyGroupByCustomValue;

use super::{CustomValueSupport, NuSchema, PolarsPluginObject, PolarsPluginType};

// Lazyframe wrapper for Nushell operations
// Polars LazyFrame is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Clone)]
pub struct NuLazyGroupBy {
    pub id: Uuid,
    pub group_by: Arc<LazyGroupBy>,
    pub schema: NuSchema,
    pub from_eager: bool,
}

impl fmt::Debug for NuLazyGroupBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuLazyGroupBy")
    }
}

impl NuLazyGroupBy {
    pub fn new(group_by: LazyGroupBy, from_eager: bool, schema: NuSchema) -> Self {
        NuLazyGroupBy {
            id: Uuid::new_v4(),
            group_by: Arc::new(group_by),
            from_eager,
            schema,
        }
    }

    pub fn to_polars(&self) -> LazyGroupBy {
        (*self.group_by).clone()
    }
}

impl Cacheable for NuLazyGroupBy {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuLazyGroupBy(self.clone()))
    }

    fn from_cache_value(cv: PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuLazyGroupBy(df) => Ok(df),
            _ => Err(ShellError::GenericError {
                error: "Cache value is not a group by".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}

impl CustomValueSupport for NuLazyGroupBy {
    type CV = NuLazyGroupByCustomValue;

    fn custom_value(self) -> Self::CV {
        NuLazyGroupByCustomValue {
            id: self.id,
            groupby: Some(self),
        }
    }

    fn get_type_static() -> PolarsPluginType {
        PolarsPluginType::NuLazyGroupBy
    }

    fn base_value(self, _span: nu_protocol::Span) -> Result<nu_protocol::Value, ShellError> {
        Ok(Value::record(
            record! {
                "LazyGroupBy" => Value::string("apply aggregation to complete execution plan", Span::unknown())
            },
            Span::unknown(),
        ))
    }
}
