mod custom_value;

use core::fmt;
use nu_protocol::{ShellError, Span, Value};
use polars::prelude::{col, when, ChainedThen, Then};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone)]
pub enum NuWhen {
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

// Mocked deserialization of the LazyFrame object
impl<'de> Deserialize<'de> for NuWhen {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NuWhen::Then(Box::new(when(col("a")).then(col("b")))))
    }
}

impl fmt::Debug for NuWhen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuWhen")
    }
}

impl From<Then> for NuWhen {
    fn from(then: Then) -> Self {
        NuWhen::Then(Box::new(then))
    }
}

impl From<ChainedThen> for NuWhen {
    fn from(chained_when: ChainedThen) -> Self {
        NuWhen::ChainedThen(chained_when)
    }
}

impl NuWhen {
    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::Custom { val, .. } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(expr.clone()),
                None => Err(ShellError::CantConvert {
                    to_type: "when expression".into(),
                    from_type: "non when expression".into(),
                    span,
                    help: None,
                }),
            },
            x => Err(ShellError::CantConvert {
                to_type: "when expression".into(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }
}
