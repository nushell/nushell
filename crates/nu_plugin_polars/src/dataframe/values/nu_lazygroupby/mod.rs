mod custom_value;

use core::fmt;
use nu_protocol::ShellError;
use polars::prelude::{LazyGroupBy, Schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use uuid::Uuid;

use crate::{Cacheable, CustomValueSupport};

pub use self::custom_value::NuLazyGroupByCustomValue;

use super::PhysicalType;

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

    pub fn into_polars(&self) -> LazyGroupBy {
        self.group_by
            .as_ref()
            .map(|arc| (**arc).clone())
            .expect("GroupBy cannot be none to convert")
    }
}

impl Cacheable for NuLazyGroupBy {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PhysicalType, ShellError> {
        Ok(PhysicalType::NuLazyGroupBy(self.clone()))
    }

    fn from_cache_value(cv: PhysicalType) -> Result<Self, ShellError> {
        match cv {
            PhysicalType::NuLazyGroupBy(df) => Ok(df),
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

    fn type_name() -> &'static str {
        "NuLazyGroupBy"
    }
}
