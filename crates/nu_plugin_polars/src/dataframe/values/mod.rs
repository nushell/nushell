mod nu_dataframe;
mod nu_expression;
mod nu_lazyframe;
mod nu_lazygroupby;
mod nu_schema;
mod nu_when;
pub mod utils;

use std::fmt;

pub use nu_dataframe::{Axis, Column, NuDataFrame, NuDataFrameCustomValue};
pub use nu_expression::{NuExpression, NuExpressionCustomValue};
pub use nu_lazyframe::{NuLazyFrame, NuLazyFrameCustomValue};
pub use nu_lazygroupby::{NuLazyGroupBy, NuLazyGroupByCustomValue};
use nu_protocol::{CustomValue, PipelineData, ShellError, Span, Value};
pub use nu_schema::{str_to_dtype, NuSchema};
pub use nu_when::{NuWhen, NuWhenCustomValue};
use uuid::Uuid;

use crate::{CustomValueSupport, PolarsPlugin};

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
