mod file_type;
mod nu_dataframe;
mod nu_dtype;
mod nu_expression;
mod nu_lazyframe;
mod nu_lazygroupby;
mod nu_schema;
mod nu_when;
pub mod utils;

use crate::{Cacheable, PolarsPlugin};
use nu_dtype::custom_value::NuDataTypeCustomValue;
use nu_plugin::EngineInterface;
use nu_protocol::{
    CustomValue, PipelineData, ShellError, Span, Spanned, Type, Value, ast::Operator,
};
use nu_schema::custom_value::NuSchemaCustomValue;
use std::{cmp::Ordering, fmt};
use uuid::Uuid;

pub use file_type::PolarsFileType;
pub use nu_dataframe::{Axis, Column, NuDataFrame, NuDataFrameCustomValue};
pub use nu_dtype::NuDataType;
pub use nu_dtype::{datatype_list, str_to_dtype, str_to_time_unit};
pub use nu_expression::{NuExpression, NuExpressionCustomValue};
pub use nu_lazyframe::{NuLazyFrame, NuLazyFrameCustomValue};
pub use nu_lazygroupby::{NuLazyGroupBy, NuLazyGroupByCustomValue};
pub use nu_schema::NuSchema;
pub use nu_when::{NuWhen, NuWhenCustomValue, NuWhenType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolarsPluginType {
    NuDataFrame,
    NuLazyFrame,
    NuExpression,
    NuLazyGroupBy,
    NuWhen,
    NuPolarsTestData,
    NuDataType,
    NuSchema,
}

impl PolarsPluginType {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::NuDataFrame => "polars_dataframe",
            Self::NuLazyFrame => "polars_lazyframe",
            Self::NuExpression => "polars_expression",
            Self::NuLazyGroupBy => "polars_group_by",
            Self::NuWhen => "polars_when",
            Self::NuPolarsTestData => "polars_test_data",
            Self::NuDataType => "polars_datatype",
            Self::NuSchema => "polars_schema",
        }
    }

    pub fn types() -> &'static [PolarsPluginType] {
        &[
            PolarsPluginType::NuDataFrame,
            PolarsPluginType::NuLazyFrame,
            PolarsPluginType::NuExpression,
            PolarsPluginType::NuLazyGroupBy,
            PolarsPluginType::NuWhen,
            PolarsPluginType::NuDataType,
            PolarsPluginType::NuSchema,
        ]
    }
}

impl From<PolarsPluginType> for Type {
    fn from(pt: PolarsPluginType) -> Self {
        Type::Custom(pt.type_name().into())
    }
}

impl fmt::Display for PolarsPluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NuDataFrame => write!(f, "NuDataFrame"),
            Self::NuLazyFrame => write!(f, "NuLazyFrame"),
            Self::NuExpression => write!(f, "NuExpression"),
            Self::NuLazyGroupBy => write!(f, "NuLazyGroupBy"),
            Self::NuWhen => write!(f, "NuWhen"),
            Self::NuPolarsTestData => write!(f, "NuPolarsTestData"),
            Self::NuDataType => write!(f, "NuDataType"),
            Self::NuSchema => write!(f, "NuSchema"),
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
    NuPolarsTestData(Uuid, String),
    NuDataType(NuDataType),
    NuSchema(NuSchema),
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
        } else if NuSchema::can_downcast(value) {
            NuSchema::try_from_value(plugin, value).map(PolarsPluginObject::NuSchema)
        } else if NuDataType::can_downcast(value) {
            NuDataType::try_from_value(plugin, value).map(PolarsPluginObject::NuDataType)
        } else {
            Err(cant_convert_err(
                value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyFrame,
                    PolarsPluginType::NuExpression,
                    PolarsPluginType::NuLazyGroupBy,
                    PolarsPluginType::NuWhen,
                    PolarsPluginType::NuDataType,
                    PolarsPluginType::NuSchema,
                ],
            ))
        }
    }

    pub fn try_from_pipeline(
        plugin: &PolarsPlugin,
        input: PipelineData,
        span: Span,
    ) -> Result<Self, ShellError> {
        let value = input.into_value(span)?;
        Self::try_from_value(plugin, &value)
    }

    pub fn get_type(&self) -> PolarsPluginType {
        match self {
            Self::NuDataFrame(_) => PolarsPluginType::NuDataFrame,
            Self::NuLazyFrame(_) => PolarsPluginType::NuLazyFrame,
            Self::NuExpression(_) => PolarsPluginType::NuExpression,
            Self::NuLazyGroupBy(_) => PolarsPluginType::NuLazyGroupBy,
            Self::NuWhen(_) => PolarsPluginType::NuWhen,
            Self::NuPolarsTestData(_, _) => PolarsPluginType::NuPolarsTestData,
            Self::NuDataType(_) => PolarsPluginType::NuDataType,
            Self::NuSchema(_) => PolarsPluginType::NuSchema,
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            PolarsPluginObject::NuDataFrame(df) => df.id,
            PolarsPluginObject::NuLazyFrame(lf) => lf.id,
            PolarsPluginObject::NuExpression(e) => e.id,
            PolarsPluginObject::NuLazyGroupBy(lg) => lg.id,
            PolarsPluginObject::NuWhen(w) => w.id,
            PolarsPluginObject::NuPolarsTestData(id, _) => *id,
            PolarsPluginObject::NuDataType(dt) => dt.id,
            PolarsPluginObject::NuSchema(schema) => schema.id,
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        match self {
            PolarsPluginObject::NuDataFrame(df) => df.into_value(span),
            PolarsPluginObject::NuLazyFrame(lf) => lf.into_value(span),
            PolarsPluginObject::NuExpression(e) => e.into_value(span),
            PolarsPluginObject::NuLazyGroupBy(lg) => lg.into_value(span),
            PolarsPluginObject::NuWhen(w) => w.into_value(span),
            PolarsPluginObject::NuPolarsTestData(id, s) => {
                Value::string(format!("{id}:{s}"), Span::test_data())
            }
            PolarsPluginObject::NuDataType(dt) => dt.into_value(span),
            PolarsPluginObject::NuSchema(schema) => schema.into_value(span),
        }
    }

    pub fn dataframe(&self) -> Option<&NuDataFrame> {
        match self {
            PolarsPluginObject::NuDataFrame(df) => Some(df),
            _ => None,
        }
    }

    pub fn lazyframe(&self) -> Option<&NuLazyFrame> {
        match self {
            PolarsPluginObject::NuLazyFrame(lf) => Some(lf),
            _ => None,
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
    NuDataType(NuDataTypeCustomValue),
    NuSchema(NuSchemaCustomValue),
}

impl CustomValueType {
    pub fn id(&self) -> Uuid {
        match self {
            CustomValueType::NuDataFrame(df_cv) => df_cv.id,
            CustomValueType::NuLazyFrame(lf_cv) => lf_cv.id,
            CustomValueType::NuExpression(e_cv) => e_cv.id,
            CustomValueType::NuLazyGroupBy(lg_cv) => lg_cv.id,
            CustomValueType::NuWhen(w_cv) => w_cv.id,
            CustomValueType::NuDataType(dt_cv) => dt_cv.id,
            CustomValueType::NuSchema(schema_cv) => schema_cv.id,
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
        } else if let Some(w_cv) = val.as_any().downcast_ref::<NuDataTypeCustomValue>() {
            Ok(CustomValueType::NuDataType(w_cv.clone()))
        } else if let Some(w_cv) = val.as_any().downcast_ref::<NuSchemaCustomValue>() {
            Ok(CustomValueType::NuSchema(w_cv.clone()))
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

pub fn cant_convert_err(value: &Value, expected_types: &[PolarsPluginType]) -> ShellError {
    let type_string = expected_types
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
        lhs_span: Span,
        operator: Spanned<Operator>,
        _right: Value,
    ) -> Result<Value, ShellError> {
        Err(ShellError::OperatorUnsupportedType {
            op: operator.item,
            unsupported: Type::Custom(self.type_name().into()),
            op_span: operator.span,
            unsupported_span: lhs_span,
            help: None,
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
        let value = input.into_value(span)?;
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
        Ok(PipelineData::value(
            self.cache_and_to_value(plugin, engine, span)?,
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use polars::prelude::{DataType, TimeUnit, UnknownKind};
    use polars_compute::decimal::DEC128_MAX_PREC;

    use crate::command::datetime::timezone_utc;

    use super::*;

    #[test]
    fn test_dtype_str_to_schema_simple_types() {
        let dtype = "bool";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Boolean;
        assert_eq!(schema, expected);

        let dtype = "u8";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt8;
        assert_eq!(schema, expected);

        let dtype = "u16";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt16;
        assert_eq!(schema, expected);

        let dtype = "u32";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt32;
        assert_eq!(schema, expected);

        let dtype = "u64";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt64;
        assert_eq!(schema, expected);

        let dtype = "i8";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int8;
        assert_eq!(schema, expected);

        let dtype = "i16";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int16;
        assert_eq!(schema, expected);

        let dtype = "i32";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int32;
        assert_eq!(schema, expected);

        let dtype = "i64";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int64;
        assert_eq!(schema, expected);

        let dtype = "str";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::String;
        assert_eq!(schema, expected);

        let dtype = "binary";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Binary;
        assert_eq!(schema, expected);

        let dtype = "date";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Date;
        assert_eq!(schema, expected);

        let dtype = "time";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Time;
        assert_eq!(schema, expected);

        let dtype = "null";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Null;
        assert_eq!(schema, expected);

        let dtype = "unknown";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Unknown(UnknownKind::Any);
        assert_eq!(schema, expected);

        let dtype = "object";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Object("unknown");
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_dtype_str_schema_datetime() {
        let dtype = "datetime<ms, *>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Milliseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime<us, *>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Microseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime<μs, *>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Microseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime<ns, *>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Nanoseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime<ms, UTC>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Milliseconds, Some(timezone_utc()));
        assert_eq!(schema, expected);

        let dtype = "invalid";
        let schema = str_to_dtype(dtype, Span::unknown());
        assert!(schema.is_err())
    }

    #[test]
    fn test_dtype_str_schema_duration() {
        let dtype = "duration<ms>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Milliseconds);
        assert_eq!(schema, expected);

        let dtype = "duration<us>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Microseconds);
        assert_eq!(schema, expected);

        let dtype = "duration<μs>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Microseconds);
        assert_eq!(schema, expected);

        let dtype = "duration<ns>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Nanoseconds);
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_dtype_str_schema_decimal() {
        let dtype = "decimal<7,2>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Decimal(7usize, 2usize);
        assert_eq!(schema, expected);

        // "*" is not a permitted value for scale
        let dtype = "decimal<7,*>";
        let schema = str_to_dtype(dtype, Span::unknown());
        assert!(matches!(schema, Err(ShellError::GenericError { .. })));

        let dtype = "decimal<*,2>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::Decimal(DEC128_MAX_PREC, 2usize);
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_dtype_str_to_schema_list_types() {
        let dtype = "list<i32>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Int32));
        assert_eq!(schema, expected);

        let dtype = "list<duration<ms>>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Duration(TimeUnit::Milliseconds)));
        assert_eq!(schema, expected);

        let dtype = "list<datetime<ms, *>>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Datetime(TimeUnit::Milliseconds, None)));
        assert_eq!(schema, expected);

        let dtype = "list<decimal<7,2>>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Decimal(7usize, 2usize)));
        assert_eq!(schema, expected);

        let dtype = "list<decimal<*,2>>";
        let schema = str_to_dtype(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Decimal(DEC128_MAX_PREC, 2usize)));
        assert_eq!(schema, expected);

        let dtype = "list<decimal<7,*>>";
        let schema = str_to_dtype(dtype, Span::unknown());
        assert!(matches!(schema, Err(ShellError::GenericError { .. })));
    }
}
