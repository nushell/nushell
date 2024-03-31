mod custom_value;

use core::fmt;
use nu_protocol::{ShellError, Span, Value};
use polars::prelude::{ChainedThen, Then};
use serde::{Serialize, Serializer};
use uuid::Uuid;

use crate::Cacheable;

pub use self::custom_value::NuWhenCustomValue;

use super::{CustomValueSupport, PolarsPluginObject, PolarsPluginType};

#[derive(Debug, Clone)]
pub struct NuWhen {
    pub id: Uuid,
    pub when_type: NuWhenType,
}

#[derive(Clone)]
pub enum NuWhenType {
    Then(Box<Then>),
    ChainedThen(ChainedThen),
}

// Mocked serialization of the LazyFrame object
impl Serialize for NuWhen {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_none()
    }
}

impl fmt::Debug for NuWhenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuWhen")
    }
}

impl From<Then> for NuWhenType {
    fn from(then: Then) -> Self {
        NuWhenType::Then(Box::new(then))
    }
}

impl From<ChainedThen> for NuWhenType {
    fn from(chained_when: ChainedThen) -> Self {
        NuWhenType::ChainedThen(chained_when)
    }
}

impl From<NuWhenType> for NuWhen {
    fn from(when_type: NuWhenType) -> Self {
        Self::new(when_type)
    }
}

impl From<Then> for NuWhen {
    fn from(then: Then) -> Self {
        Self::new(then.into())
    }
}

impl From<ChainedThen> for NuWhen {
    fn from(chained_then: ChainedThen) -> Self {
        Self::new(chained_then.into())
    }
}

impl NuWhen {
    pub fn new(when_type: NuWhenType) -> Self {
        Self {
            id: Uuid::new_v4(),
            when_type,
        }
    }
}

impl Cacheable for NuWhen {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuWhen(self.clone()))
    }

    fn from_cache_value(cv: PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuWhen(when) => Ok(when),
            _ => Err(ShellError::GenericError {
                error: "Cache value is not a dataframe".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}

impl CustomValueSupport for NuWhen {
    type CV = NuWhenCustomValue;

    fn custom_value(self) -> Self::CV {
        NuWhenCustomValue {
            id: self.id,
            when: Some(self),
        }
    }

    fn get_type_static() -> PolarsPluginType {
        PolarsPluginType::NuWhen
    }

    fn base_value(self, _span: nu_protocol::Span) -> Result<nu_protocol::Value, ShellError> {
        let val: String = match self.when_type {
            NuWhenType::Then(_) => "whenthen".into(),
            NuWhenType::ChainedThen(_) => "whenthenthen".into(),
        };

        let value = Value::string(val, Span::unknown());
        Ok(value)
    }
}
