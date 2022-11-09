mod custom_value;

use core::fmt;
use nu_protocol::{ShellError, Span, Value};
use polars::prelude::{col, when, WhenThen, WhenThenThen};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone)]
pub enum NuWhen {
    WhenThen(Box<WhenThen>),
    WhenThenThen(WhenThenThen),
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
        Ok(NuWhen::WhenThen(Box::new(when(col("a")).then(col("b")))))
    }
}

impl fmt::Debug for NuWhen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuWhen")
    }
}

impl From<WhenThen> for NuWhen {
    fn from(when_then: WhenThen) -> Self {
        NuWhen::WhenThen(Box::new(when_then))
    }
}

impl From<WhenThenThen> for NuWhen {
    fn from(when_then_then: WhenThenThen) -> Self {
        NuWhen::WhenThenThen(when_then_then)
    }
}

impl NuWhen {
    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(expr.clone()),
                None => Err(ShellError::CantConvert(
                    "when expression".into(),
                    "non when expression".into(),
                    span,
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "when expression".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }
}
