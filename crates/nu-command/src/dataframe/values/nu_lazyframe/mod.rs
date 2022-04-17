mod custom_value;

use super::{NuDataFrame, NuExpression};
use core::fmt;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::prelude::{Expr, IntoLazy, LazyFrame};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Lazyframe wrapper for Nushell operations
// Polars LazyFrame is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default)]
pub struct NuLazyFrame(Option<LazyFrame>);

// Mocked serialization of the LazyFrame object
impl Serialize for NuLazyFrame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_none()
    }
}

// Mocked deserialization of the LazyFrame object
impl<'de> Deserialize<'de> for NuLazyFrame {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(NuLazyFrame::default())
    }
}

impl fmt::Debug for NuLazyFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuLazyframe")
    }
}

// Referenced access to the real LazyFrame
impl AsRef<LazyFrame> for NuLazyFrame {
    fn as_ref(&self) -> &polars::prelude::LazyFrame {
        // The only case when there cannot be a lazy frame is if it is created
        // using the default function or if created by deserializing something
        self.0.as_ref().expect("there should always be a frame")
    }
}

impl AsMut<LazyFrame> for NuLazyFrame {
    fn as_mut(&mut self) -> &mut polars::prelude::LazyFrame {
        // The only case when there cannot be a lazy frame is if it is created
        // using the default function or if created by deserializing something
        self.0.as_mut().expect("there should always be a frame")
    }
}

impl From<LazyFrame> for NuLazyFrame {
    fn from(lazy_frame: LazyFrame) -> Self {
        Self(Some(lazy_frame))
    }
}

impl NuLazyFrame {
    pub fn from_dataframe(df: NuDataFrame) -> Self {
        let lazy = df.as_ref().clone().lazy();
        Self(Some(lazy))
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn into_polars(self) -> LazyFrame {
        self.0.expect("lazyframe cannot be none to convert")
    }

    pub fn collect(self, span: Span) -> Result<NuDataFrame, ShellError> {
        self.0
            .expect("No empty lazy for collect")
            .collect()
            .map_err(|e| {
                ShellError::SpannedLabeledError(
                    "Error collecting lazy frame".to_string(),
                    e.to_string(),
                    span,
                )
            })
            .map(NuDataFrame::new)
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<NuLazyFrame>() {
                Some(expr) => Ok(Self(expr.0.clone())),
                None => Err(ShellError::CantConvert(
                    "lazy frame".into(),
                    "non-dataframe".into(),
                    span,
                )),
            },
            x => Err(ShellError::CantConvert(
                "lazy frame".into(),
                x.get_type().to_string(),
                x.span()?,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn apply<F>(self, f: F) -> Self
    where
        F: Fn(LazyFrame) -> LazyFrame,
    {
        let df = self.0.expect("Lazy frame must not be empty to apply");
        let new_frame = f(df);

        new_frame.into()
    }

    pub fn apply_with_expr<F>(self, expr: NuExpression, f: F) -> Self
    where
        F: Fn(LazyFrame, Expr) -> LazyFrame,
    {
        let df = self.0.expect("Lazy frame must not be empty to apply");
        let expr = expr.into_polars();
        let new_frame = f(df, expr);

        new_frame.into()
    }
}
