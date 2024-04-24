mod nu_dataframe;
mod nu_expression;
mod nu_lazyframe;
mod nu_lazygroupby;
mod nu_schema;
mod nu_when;
pub mod utils;

use std::{cmp::Ordering, fmt};

pub use nu_dataframe::{Axis, Column, NuDataFrame, NuDataFrameCustomValue};
pub use nu_expression::{NuExpression, NuExpressionCustomValue};
pub use nu_lazyframe::{NuLazyFrame, NuLazyFrameCustomValue};
pub use nu_lazygroupby::{NuLazyGroupBy, NuLazyGroupByCustomValue};
use nu_plugin::EngineInterface;
use nu_protocol::{ast::Operator, CustomValue, PipelineData, ShellError, Span, Spanned, Value};
pub use nu_schema::{str_to_dtype, NuSchema};
pub use nu_when::{NuWhen, NuWhenCustomValue, NuWhenType};
use uuid::Uuid;

use crate::{Cacheable, PolarsPlugin};

#[derive(Debug, Clone)]
pub enum PolarsPluginType {
    NuDataFrame,
    NuLazyFrame,
    NuExpression,
    NuLazyGroupBy,
    NuWhen,
}

impl fmt::Display for PolarsPluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NuDataFrame => write!(f, "NuDataFrame"),
            Self::NuLazyFrame => write!(f, "NuLazyFrame"),
            Self::NuExpression => write!(f, "NuExpression"),
            Self::NuLazyGroupBy => write!(f, "NuLazyGroupBy"),
            Self::NuWhen => write!(f, "NuWhen"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PolarsPluginObject {
    NuDataFrame(NuDataFrame),
    NuLazyFrame(NuLazyFrame),
    NuExpression(NuExpression),
    NuLazyGroupBy(NuLazyGroupBy),
    NuWhen(NuWhen),
}

impl PolarsPluginObject {
    pub fn try_from_value(
        plugin: &PolarsPlugin,
        value: &Value,
    ) -> Result<PolarsPluginObject, ShellError> {
        if NuDataFrame::can_downcast(value) {
            NuDataFrame::try_from_value(plugin, value).map(PolarsPluginObject::NuDataFrame)
        } else if NuLazyFrame::can_downcast(value) {
            NuLazyFrame::try_from_value(plugin, value).map(PolarsPluginObject::NuLazyFrame)
        } else if NuExpression::can_downcast(value) {
            NuExpression::try_from_value(plugin, value).map(PolarsPluginObject::NuExpression)
        } else if NuLazyGroupBy::can_downcast(value) {
            NuLazyGroupBy::try_from_value(plugin, value).map(PolarsPluginObject::NuLazyGroupBy)
        } else if NuWhen::can_downcast(value) {
            NuWhen::try_from_value(plugin, value).map(PolarsPluginObject::NuWhen)
        } else {
            Err(cant_convert_err(
                value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyFrame,
                    PolarsPluginType::NuExpression,
                    PolarsPluginType::NuLazyGroupBy,
                    PolarsPluginType::NuWhen,
                ],
            ))
        }
    }

    pub fn try_from_pipeline(
        plugin: &PolarsPlugin,
        input: PipelineData,
        span: Span,
    ) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(plugin, &value)
    }

    pub fn get_type(&self) -> PolarsPluginType {
        match self {
            Self::NuDataFrame(_) => PolarsPluginType::NuDataFrame,
            Self::NuLazyFrame(_) => PolarsPluginType::NuLazyFrame,
            Self::NuExpression(_) => PolarsPluginType::NuExpression,
            Self::NuLazyGroupBy(_) => PolarsPluginType::NuLazyGroupBy,
            Self::NuWhen(_) => PolarsPluginType::NuWhen,
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            PolarsPluginObject::NuDataFrame(df) => df.id,
            PolarsPluginObject::NuLazyFrame(lf) => lf.id,
            PolarsPluginObject::NuExpression(e) => e.id,
            PolarsPluginObject::NuLazyGroupBy(lg) => lg.id,
            PolarsPluginObject::NuWhen(w) => w.id,
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        match self {
            PolarsPluginObject::NuDataFrame(df) => df.into_value(span),
            PolarsPluginObject::NuLazyFrame(lf) => lf.into_value(span),
            PolarsPluginObject::NuExpression(e) => e.into_value(span),
            PolarsPluginObject::NuLazyGroupBy(lg) => lg.into_value(span),
            PolarsPluginObject::NuWhen(w) => w.into_value(span),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CustomValueType {
    NuDataFrame(NuDataFrameCustomValue),
    NuLazyFrame(NuLazyFrameCustomValue),
    NuExpression(NuExpressionCustomValue),
    NuLazyGroupBy(NuLazyGroupByCustomValue),
    NuWhen(NuWhenCustomValue),
}

impl CustomValueType {
    pub fn id(&self) -> Uuid {
        match self {
            CustomValueType::NuDataFrame(df_cv) => df_cv.id,
            CustomValueType::NuLazyFrame(lf_cv) => lf_cv.id,
            CustomValueType::NuExpression(e_cv) => e_cv.id,
            CustomValueType::NuLazyGroupBy(lg_cv) => lg_cv.id,
            CustomValueType::NuWhen(w_cv) => w_cv.id,
        }
    }

    pub fn try_from_custom_value(val: Box<dyn CustomValue>) -> Result<CustomValueType, ShellError> {
        if let Some(df_cv) = val.as_any().downcast_ref::<NuDataFrameCustomValue>() {
            Ok(CustomValueType::NuDataFrame(df_cv.clone()))
        } else if let Some(lf_cv) = val.as_any().downcast_ref::<NuLazyFrameCustomValue>() {
            Ok(CustomValueType::NuLazyFrame(lf_cv.clone()))
        } else if let Some(e_cv) = val.as_any().downcast_ref::<NuExpressionCustomValue>() {
            Ok(CustomValueType::NuExpression(e_cv.clone()))
        } else if let Some(lg_cv) = val.as_any().downcast_ref::<NuLazyGroupByCustomValue>() {
            Ok(CustomValueType::NuLazyGroupBy(lg_cv.clone()))
        } else if let Some(w_cv) = val.as_any().downcast_ref::<NuWhenCustomValue>() {
            Ok(CustomValueType::NuWhen(w_cv.clone()))
        } else {
            Err(ShellError::CantConvert {
                to_type: "physical type".into(),
                from_type: "value".into(),
                span: Span::unknown(),
                help: None,
            })
        }
    }
}

pub fn cant_convert_err(value: &Value, types: &[PolarsPluginType]) -> ShellError {
    let type_string = types
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
        .join(", ");

    ShellError::CantConvert {
        to_type: type_string,
        from_type: value.get_type().to_string(),
        span: value.span(),
        help: None,
    }
}

pub trait PolarsPluginCustomValue: CustomValue {
    type PolarsPluginObjectType: Clone;

    fn id(&self) -> &Uuid;

    fn internal(&self) -> &Option<Self::PolarsPluginObjectType>;

    fn custom_value_to_base_value(
        &self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
    ) -> Result<Value, ShellError>;

    fn custom_value_operation(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        _lhs_span: Span,
        operator: Spanned<Operator>,
        _right: Value,
    ) -> Result<Value, ShellError> {
        Err(ShellError::UnsupportedOperator {
            operator: operator.item,
            span: operator.span,
        })
    }

    fn custom_value_follow_path_int(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        self_span: Span,
        _index: Spanned<usize>,
    ) -> Result<Value, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: self_span,
        })
    }

    fn custom_value_follow_path_string(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        self_span: Span,
        _column_name: Spanned<String>,
    ) -> Result<Value, ShellError> {
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: self_span,
        })
    }

    fn custom_value_partial_cmp(
        &self,
        _plugin: &PolarsPlugin,
        _engine: &EngineInterface,
        _other_value: Value,
    ) -> Result<Option<Ordering>, ShellError> {
        Ok(None)
    }
}

/// Handles the ability for a PolarsObjectType implementations to convert between
/// their respective CustValue type.
/// PolarsPluginObjectType's (NuDataFrame, NuLazyFrame) should
/// implement this trait.  
pub trait CustomValueSupport: Cacheable {
    type CV: PolarsPluginCustomValue<PolarsPluginObjectType = Self> + CustomValue + 'static;

    fn get_type(&self) -> PolarsPluginType {
        Self::get_type_static()
    }

    fn get_type_static() -> PolarsPluginType;

    fn custom_value(self) -> Self::CV;

    fn base_value(self, span: Span) -> Result<Value, ShellError>;

    fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self.custom_value()), span)
    }

    fn try_from_custom_value(plugin: &PolarsPlugin, cv: &Self::CV) -> Result<Self, ShellError> {
        if let Some(internal) = cv.internal() {
            Ok(internal.clone())
        } else {
            Self::get_cached(plugin, cv.id())?.ok_or_else(|| ShellError::GenericError {
                error: format!("Dataframe {:?} not found in cache", cv.id()),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    }

    fn try_from_value(plugin: &PolarsPlugin, value: &Value) -> Result<Self, ShellError> {
        if let Value::Custom { val, .. } = value {
            if let Some(cv) = val.as_any().downcast_ref::<Self::CV>() {
                Self::try_from_custom_value(plugin, cv)
            } else {
                Err(ShellError::CantConvert {
                    to_type: Self::get_type_static().to_string(),
                    from_type: value.get_type().to_string(),
                    span: value.span(),
                    help: None,
                })
            }
        } else {
            Err(ShellError::CantConvert {
                to_type: Self::get_type_static().to_string(),
                from_type: value.get_type().to_string(),
                span: value.span(),
                help: None,
            })
        }
    }

    fn try_from_pipeline(
        plugin: &PolarsPlugin,
        input: PipelineData,
        span: Span,
    ) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(plugin, &value)
    }

    fn can_downcast(value: &Value) -> bool {
        if let Value::Custom { val, .. } = value {
            val.as_any().downcast_ref::<Self::CV>().is_some()
        } else {
            false
        }
    }

    /// Wraps the cache and into_value calls.
    /// This function also does mapping back and forth
    /// between lazy and eager values and makes sure they
    /// are cached appropriately.
    fn cache_and_to_value(
        self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        span: Span,
    ) -> Result<Value, ShellError> {
        match self.to_cache_value()? {
            // if it was from a lazy value, make it lazy again
            PolarsPluginObject::NuDataFrame(df) if df.from_lazy => {
                let df = df.lazy();
                Ok(df.cache(plugin, engine, span)?.into_value(span))
            }
            // if it was from an eager value, make it eager again
            PolarsPluginObject::NuLazyFrame(lf) if lf.from_eager => {
                let lf = lf.collect(span)?;
                Ok(lf.cache(plugin, engine, span)?.into_value(span))
            }
            _ => Ok(self.cache(plugin, engine, span)?.into_value(span)),
        }
    }

    /// Caches the object, converts it to a it's CustomValue counterpart
    /// And creates a pipeline data object out of it
    #[inline]
    fn to_pipeline_data(
        self,
        plugin: &PolarsPlugin,
        engine: &EngineInterface,
        span: Span,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::Value(
            self.cache_and_to_value(plugin, engine, span)?,
            None,
        ))
    }
}
