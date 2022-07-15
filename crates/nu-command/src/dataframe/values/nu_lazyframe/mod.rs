mod custom_value;

use super::{NuDataFrame, NuExpression};
use core::fmt;
use nu_protocol::{PipelineData, ShellError, Span, Value};
use polars::prelude::{Expr, IntoLazy, LazyFrame, Schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Lazyframe wrapper for Nushell operations
// Polars LazyFrame is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default)]
pub struct NuLazyFrame {
    pub lazy: Option<LazyFrame>,
    pub schema: Option<Schema>,
    pub from_eager: bool,
}

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
        self.lazy.as_ref().expect("there should always be a frame")
    }
}

impl AsMut<LazyFrame> for NuLazyFrame {
    fn as_mut(&mut self) -> &mut polars::prelude::LazyFrame {
        // The only case when there cannot be a lazy frame is if it is created
        // using the default function or if created by deserializing something
        self.lazy.as_mut().expect("there should always be a frame")
    }
}

impl From<LazyFrame> for NuLazyFrame {
    fn from(lazy_frame: LazyFrame) -> Self {
        Self {
            lazy: Some(lazy_frame),
            from_eager: false,
            schema: None,
        }
    }
}

impl NuLazyFrame {
    pub fn new(from_eager: bool, lazy: LazyFrame) -> Self {
        Self {
            lazy: Some(lazy),
            from_eager,
            schema: None,
        }
    }

    pub fn from_dataframe(df: NuDataFrame) -> Self {
        let lazy = df.as_ref().clone().lazy();
        Self {
            lazy: Some(lazy),
            from_eager: true,
            schema: Some(df.as_ref().schema()),
        }
    }

    pub fn into_value(self, span: Span) -> Result<Value, ShellError> {
        if self.from_eager {
            let df = self.collect(span)?;
            Ok(Value::CustomValue {
                val: Box::new(df),
                span,
            })
        } else {
            Ok(Value::CustomValue {
                val: Box::new(self),
                span,
            })
        }
    }

    pub fn into_polars(self) -> LazyFrame {
        self.lazy.expect("lazyframe cannot be none to convert")
    }

    pub fn collect(self, span: Span) -> Result<NuDataFrame, ShellError> {
        self.lazy
            .expect("No empty lazy for collect")
            .collect()
            .map_err(|e| {
                ShellError::GenericError(
                    "Error collecting lazy frame".to_string(),
                    e.to_string(),
                    Some(span),
                    None,
                    Vec::new(),
                )
            })
            .map(|df| NuDataFrame {
                df,
                from_lazy: !self.from_eager,
            })
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        if Self::can_downcast(&value) {
            Ok(Self::get_lazy_df(value)?)
        } else if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(value)?;
            Ok(NuLazyFrame::from_dataframe(df))
        } else {
            Err(ShellError::CantConvert(
                "lazy or eager dataframe".into(),
                value.get_type().to_string(),
                value.span()?,
                None,
            ))
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn get_lazy_df(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(Self {
                    lazy: expr.lazy.clone(),
                    from_eager: false,
                    schema: None,
                }),
                None => Err(ShellError::CantConvert(
                    "lazy frame".into(),
                    "non-dataframe".into(),
                    span,
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "lazy frame".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn can_downcast(value: &Value) -> bool {
        if let Value::CustomValue { val, .. } = value {
            val.as_any().downcast_ref::<Self>().is_some()
        } else {
            false
        }
    }

    pub fn apply_with_expr<F>(self, expr: NuExpression, f: F) -> Self
    where
        F: Fn(LazyFrame, Expr) -> LazyFrame,
    {
        let df = self.lazy.expect("Lazy frame must not be empty to apply");
        let expr = expr.into_polars();
        let new_frame = f(df, expr);

        Self {
            from_eager: self.from_eager,
            lazy: Some(new_frame),
            schema: None,
        }
    }
}
