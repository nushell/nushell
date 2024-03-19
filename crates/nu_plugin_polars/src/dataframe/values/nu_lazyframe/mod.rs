mod custom_value;

use crate::DataFrameCache;

use super::{NuDataFrame, NuExpression};
use core::fmt;
use nu_plugin::EngineInterface;
use nu_protocol::{record, PipelineData, ShellError, Span, Value};
use polars::prelude::{Expr, IntoLazy, LazyFrame, Schema};
use uuid::Uuid;

pub use custom_value::NuLazyFrameCustomValue;

// Lazyframe wrapper for Nushell operations
// Polars LazyFrame is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default, Clone)]
pub struct NuLazyFrame {
    pub id: Uuid,
    pub lazy: Option<LazyFrame>,
    pub schema: Option<Schema>,
    pub from_eager: bool,
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
        NuLazyFrame::new(false, lazy_frame)
    }
}

impl NuLazyFrame {
    pub fn new(from_eager: bool, lazy: LazyFrame) -> Self {
        Self::new_with_option_lazy(from_eager, Some(lazy))
    }

    fn new_with_option_lazy(from_eager: bool, lazy: Option<LazyFrame>) -> Self {
        Self {
            id: Uuid::new_v4(),
            lazy,
            from_eager,
            schema: None,
        }
    }

    pub fn from_dataframe(df: NuDataFrame) -> Self {
        let lazy = df.as_ref().clone().lazy();
        NuLazyFrame::new(true, lazy)
    }

    pub fn into_value(self, span: Span) -> Result<Value, ShellError> {
        if self.from_eager {
            let df = self.collect(span)?;
            Ok(Value::custom_value(Box::new(df.custom_value()), span))
        } else {
            Ok(Value::custom_value(Box::new(self.custom_value()), span))
        }
    }

    pub fn custom_value(self) -> NuLazyFrameCustomValue {
        self.into()
    }

    pub fn base_value(&self, span: Span) -> Result<Value, ShellError> {
        let optimized_plan = self
            .as_ref()
            .describe_optimized_plan()
            .unwrap_or_else(|_| "<NOT AVAILABLE>".to_string());
        Ok(Value::record(
            record! {
                "plan" => Value::string(self.as_ref().describe_plan(), span),
                "optimized_plan" => Value::string(optimized_plan, span),
            },
            span,
        ))
    }

    pub fn into_polars(self) -> LazyFrame {
        self.lazy.expect("lazyframe cannot be none to convert")
    }

    pub fn collect(self, span: Span) -> Result<NuDataFrame, ShellError> {
        self.lazy
            .expect("No empty lazy for collect")
            .collect()
            .map_err(|e| ShellError::GenericError {
                error: "Error collecting lazy frame".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            })
            .map(|df| NuDataFrame::new(!self.from_eager, df))
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        if Self::can_downcast(&value) {
            Ok(Self::get_lazy_df(value)?)
        } else if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(value)?;
            Ok(NuLazyFrame::from_dataframe(df))
        } else {
            Err(ShellError::CantConvert {
                to_type: "lazy or eager dataframe".into(),
                from_type: value.get_type().to_string(),
                span: value.span(),
                help: None,
            })
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn get_lazy_df(value: Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::CustomValue { val, .. } => {
                match val.as_any().downcast_ref::<NuLazyFrameCustomValue>() {
                    Some(expr) => NuLazyFrame::try_from(expr),
                    None => Err(ShellError::CantConvert {
                        to_type: "lazy frame".into(),
                        from_type: "non-dataframe".into(),
                        span,
                        help: None,
                    }),
                }
            }
            x => Err(ShellError::CantConvert {
                to_type: "lazy frame".into(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }

    pub fn can_downcast(value: &Value) -> bool {
        if let Value::CustomValue { val, .. } = value {
            val.as_any()
                .downcast_ref::<NuLazyFrameCustomValue>()
                .is_some()
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
        Self::new(self.from_eager, new_frame)
    }

    pub fn insert_cache(self, engine: &EngineInterface) -> Result<Self, ShellError> {
        DataFrameCache::insert_lazy(engine, self.clone()).map(|_| self)
    }
}
