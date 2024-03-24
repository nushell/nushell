mod custom_value;

use crate::{Cacheable, CustomValueSupport};

use super::{NuDataFrame, NuExpression, PhysicalType};
use core::fmt;
use nu_protocol::{record, ShellError, Span, Value};
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

    pub fn apply_with_expr<F>(self, expr: NuExpression, f: F) -> Self
    where
        F: Fn(LazyFrame, Expr) -> LazyFrame,
    {
        let df = self.lazy.expect("Lazy frame must not be empty to apply");
        let expr = expr.into_polars();
        let new_frame = f(df, expr);
        Self::new(self.from_eager, new_frame)
    }
}

impl Cacheable for NuLazyFrame {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PhysicalType, ShellError> {
        Ok(PhysicalType::NuLazyFrame(self.clone()))
    }

    fn from_cache_value(cv: PhysicalType) -> Result<Self, ShellError> {
        match cv {
            PhysicalType::NuLazyFrame(df) => Ok(df),
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
}
