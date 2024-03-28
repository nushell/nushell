mod custom_value;

use crate::Cacheable;

use super::{
    CustomValueSupport, NuDataFrame, NuExpression, NuSchema, PolarsPluginObject, PolarsPluginType,
};
use core::fmt;
use nu_protocol::{record, ShellError, Span, Value};
use polars::prelude::{Expr, IntoLazy, LazyFrame};
use std::sync::Arc;
use uuid::Uuid;

pub use custom_value::NuLazyFrameCustomValue;

// Lazyframe wrapper for Nushell operations
// Polars LazyFrame is behind and Option to allow easy implementation of
// the Deserialize trait
#[derive(Default, Clone)]
pub struct NuLazyFrame {
    pub id: Uuid,
    pub lazy: Arc<LazyFrame>,
    pub from_eager: bool,
}

impl fmt::Debug for NuLazyFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NuLazyframe")
    }
}

impl From<LazyFrame> for NuLazyFrame {
    fn from(lazy_frame: LazyFrame) -> Self {
        NuLazyFrame::new(false, lazy_frame)
    }
}

impl NuLazyFrame {
    pub fn new(from_eager: bool, lazy: LazyFrame) -> Self {
        Self {
            id: Uuid::new_v4(),
            lazy: Arc::new(lazy),
            from_eager,
        }
    }

    pub fn get_type() -> PolarsPluginType {
        PolarsPluginType::NuLazyFrame
    }

    pub fn from_dataframe(df: NuDataFrame) -> Self {
        let lazy = df.as_ref().clone().lazy();
        NuLazyFrame::new(true, lazy)
    }

    pub fn to_polars(&self) -> LazyFrame {
        (*self.lazy).clone()
    }

    pub fn collect(self, span: Span) -> Result<NuDataFrame, ShellError> {
        self.to_polars()
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

    pub fn apply_with_expr<F>(self, expr: NuExpression, f: F) -> Self
    where
        F: Fn(LazyFrame, Expr) -> LazyFrame,
    {
        let df = self.to_polars();
        let expr = expr.to_polars();
        let new_frame = f(df, expr);
        Self::new(self.from_eager, new_frame)
    }

    pub fn schema(&self) -> Result<NuSchema, ShellError> {
        let internal_schema = self.lazy.schema().map_err(|e| ShellError::GenericError {
            error: "Error getting schema from lazy frame".into(),
            msg: e.to_string(),
            span: None,
            help: None,
            inner: vec![],
        })?;
        Ok(internal_schema.into())
    }
}

impl Cacheable for NuLazyFrame {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuLazyFrame(self.clone()))
    }

    fn from_cache_value(cv: PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuLazyFrame(df) => Ok(df),
            _ => Err(ShellError::GenericError {
                error: "Cache value is not a lazyframe".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}

impl CustomValueSupport for NuLazyFrame {
    type CV = NuLazyFrameCustomValue;

    fn custom_value(self) -> Self::CV {
        NuLazyFrameCustomValue {
            id: self.id,
            lazyframe: Some(self),
        }
    }

    fn type_name() -> &'static str {
        "NULazyFrame"
    }

    fn base_value(self, span: Span) -> Result<Value, ShellError> {
        let optimized_plan = self
            .lazy
            .describe_optimized_plan()
            .unwrap_or_else(|_| "<NOT AVAILABLE>".to_string());
        Ok(Value::record(
            record! {
                "plan" => Value::string(self.lazy.describe_plan(), span),
                "optimized_plan" => Value::string(optimized_plan, span),
            },
            span,
        ))
    }
}
