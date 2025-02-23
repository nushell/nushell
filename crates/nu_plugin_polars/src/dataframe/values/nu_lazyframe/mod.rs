mod custom_value;

use crate::{Cacheable, PolarsPlugin};

use super::{
    cant_convert_err, CustomValueSupport, NuDataFrame, NuExpression, NuSchema, PolarsPluginObject,
    PolarsPluginType,
};
use core::fmt;
use nu_protocol::{record, PipelineData, ShellError, Span, Value};
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

    pub fn from_dataframe(df: NuDataFrame) -> Self {
        let lazy = df.as_ref().clone().lazy();
        NuLazyFrame::new(true, lazy)
    }

    pub fn to_polars(&self) -> LazyFrame {
        (*self.lazy).clone()
    }

    pub fn collect(self, span: Span) -> Result<NuDataFrame, ShellError> {
        crate::handle_panic(
            || {
                self.to_polars()
                    .collect()
                    .map_err(|e| ShellError::GenericError {
                        error: "Error collecting lazy frame".into(),
                        msg: e.to_string(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    })
                    .map(|df| NuDataFrame::new(true, df))
            },
            span,
        )
    }

    pub fn apply_with_expr<F>(self, expr: NuExpression, f: F) -> Self
    where
        F: Fn(LazyFrame, Expr) -> LazyFrame,
    {
        let df = self.to_polars();
        let expr = expr.into_polars();
        let new_frame = f(df, expr);
        Self::new(self.from_eager, new_frame)
    }

    pub fn schema(&mut self) -> Result<NuSchema, ShellError> {
        let internal_schema = Arc::make_mut(&mut self.lazy)
            .collect_schema()
            .map_err(|e| ShellError::GenericError {
                error: "Error getting schema from lazy frame".into(),
                msg: e.to_string(),
                span: None,
                help: None,
                inner: vec![],
            })?;
        Ok(internal_schema.into())
    }

    /// Get a NuLazyFrame from the value. This differs from try_from_value as it will coerce a
    /// NuDataFrame into a NuLazyFrame
    pub fn try_from_value_coerce(
        plugin: &PolarsPlugin,
        value: &Value,
    ) -> Result<NuLazyFrame, ShellError> {
        match PolarsPluginObject::try_from_value(plugin, value)? {
            PolarsPluginObject::NuDataFrame(df) => Ok(df.lazy()),
            PolarsPluginObject::NuLazyFrame(lazy) => Ok(lazy),
            _ => Err(cant_convert_err(
                value,
                &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
            )),
        }
    }

    /// This differs from try_from_pipeline as it will attempt to coerce the type into a NuDataFrame.
    /// So, if the pipeline type is a NuLazyFrame it will be collected and returned as NuDataFrame.
    pub fn try_from_pipeline_coerce(
        plugin: &PolarsPlugin,
        input: PipelineData,
        span: Span,
    ) -> Result<Self, ShellError> {
        let value = input.into_value(span)?;
        Self::try_from_value_coerce(plugin, &value)
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

    fn get_type_static() -> PolarsPluginType {
        PolarsPluginType::NuLazyFrame
    }

    fn base_value(self, span: Span) -> Result<Value, ShellError> {
        let optimized_plan = self
            .lazy
            .describe_optimized_plan()
            .unwrap_or_else(|_| "<NOT AVAILABLE>".to_string());
        Ok(Value::record(
            record! {
                "plan" => Value::string(
                    self.lazy.describe_plan().map_err(|e| ShellError::GenericError {
                        error: "Error getting plan".into(),
                        msg: e.to_string(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    })?,
                    span),
                "optimized_plan" => Value::string(optimized_plan, span),
            },
            span,
        ))
    }
}
