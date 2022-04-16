mod custom_value;

use core::fmt;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::prelude::LazyGroupBy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Lazyframe wrapper for Nushell operations
// Polars LazyFrame is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default)]
pub struct NuLazyGroupBy(Option<LazyGroupBy>);

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
        self.0.as_ref().expect("there should always be a frame")
    }
}

impl AsMut<LazyGroupBy> for NuLazyGroupBy {
    fn as_mut(&mut self) -> &mut polars::prelude::LazyGroupBy {
        // The only case when there cannot be a lazy frame is if it is created
        // using the default function or if created by deserializing something
        self.0.as_mut().expect("there should always be a frame")
    }
}

impl From<LazyGroupBy> for NuLazyGroupBy {
    fn from(group_by: LazyGroupBy) -> Self {
        Self(Some(group_by))
    }
}

impl NuLazyGroupBy {
    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn into_polars(self) -> LazyGroupBy {
        self.0.expect("GroupBy cannot be none to convert")
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => {
                match val.as_any().downcast_ref::<NuLazyGroupBy>() {
                    Some(group) => Ok(Self(group.0.clone())),
                    None => Err(ShellError::CantConvert(
                        "lazy frame".into(),
                        "non-dataframe".into(),
                        span,
                    )),
                }
            }
            x => Err(ShellError::CantConvert(
                "lazy groupby".into(),
                x.get_type().to_string(),
                x.span()?,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }
}
