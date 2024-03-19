mod custom_value;

use core::fmt;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::prelude::{LazyGroupBy, Schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use uuid::Uuid;

use crate::DataFrameCache;

pub use self::custom_value::NuLazyGroupByCustomValue;

// Lazyframe wrapper for Nushell operations
// Polars LazyFrame is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default, Clone)]
pub struct NuLazyGroupBy {
    pub id: Uuid,
    pub group_by: Option<Arc<LazyGroupBy>>,
    pub schema: Option<Arc<Schema>>,
    pub from_eager: bool,
}

// Mocked serialization of the LazyFrame object
impl Serialize for NuLazyGroupBy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_none()
    }
}

// Mocked deserialization of the LazyFrame object
impl<'de> Deserialize<'de> for NuLazyGroupBy {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NuLazyGroupBy::default())
    }
}

impl fmt::Debug for NuLazyGroupBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuLazyGroupBy")
    }
}

// Referenced access to the real LazyFrame
impl AsRef<LazyGroupBy> for NuLazyGroupBy {
    fn as_ref(&self) -> &polars::prelude::LazyGroupBy {
        // The only case when there cannot be a lazy frame is if it is created
        // using the default function or if created by deserializing something
        self.group_by
            .as_ref()
            .expect("there should always be a frame")
    }
}

impl From<LazyGroupBy> for NuLazyGroupBy {
    fn from(group_by: LazyGroupBy) -> Self {
        NuLazyGroupBy::new(Some(group_by), false, None)
    }
}

impl NuLazyGroupBy {
    pub fn new(group_by: Option<LazyGroupBy>, from_eager: bool, schema: Option<Schema>) -> Self {
        NuLazyGroupBy {
            id: Uuid::new_v4(),
            group_by: group_by.map(Arc::new),
            from_eager,
            schema: schema.map(Arc::new),
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::custom_value(Box::new(self.custom_value()), span)
    }

    pub fn custom_value(self) -> NuLazyGroupByCustomValue {
        self.into()
    }

    pub fn into_polars(&self) -> LazyGroupBy {
        self.group_by
            .as_ref()
            .map(|arc| (**arc).clone())
            .expect("GroupBy cannot be none to convert")
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::CustomValue { val, .. } => {
                match val.as_any().downcast_ref::<NuLazyGroupByCustomValue>() {
                    Some(group) => Self::try_from(group),
                    None => Err(ShellError::CantConvert {
                        to_type: "lazy groupby".into(),
                        from_type: "custom value".into(),
                        span,
                        help: None,
                    }),
                }
            }
            x => Err(ShellError::CantConvert {
                to_type: "lazy groupby".into(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn insert_cache(self) -> Self {
        DataFrameCache::instance().insert_group_by(self.clone());
        self
    }
}
