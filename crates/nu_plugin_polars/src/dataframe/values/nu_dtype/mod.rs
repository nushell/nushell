pub mod custom_value;

use custom_value::NuDataTypeCustomValue;
use nu_protocol::{ShellError, Span, Value, record};
use polars::prelude::{DataType, Field, TimeUnit, UnknownKind};
use uuid::Uuid;

use crate::{Cacheable, PolarsPlugin, command::datetime::timezone_from_str};

use super::{CustomValueSupport, PolarsPluginObject, PolarsPluginType};

#[derive(Debug, Clone)]
pub struct NuDataType {
    pub id: uuid::Uuid,
    dtype: DataType,
}

impl NuDataType {
    pub fn new(dtype: DataType) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            dtype,
        }
    }

    pub fn new_with_str(dtype: &str, span: Span) -> Result<Self, ShellError> {
        let dtype = str_to_dtype(dtype, span)?;
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            dtype,
        })
    }

    pub fn to_polars(&self) -> DataType {
        self.dtype.clone()
    }
}

impl From<NuDataType> for Value {
    fn from(nu_dtype: NuDataType) -> Self {
        Value::String {
            val: nu_dtype.dtype.to_string(),
            internal_span: Span::unknown(),
        }
    }
}

impl Cacheable for NuDataType {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<super::PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuDataType(self.clone()))
    }

    fn from_cache_value(cv: super::PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuDataType(dt) => Ok(dt),
            _ => Err(ShellError::GenericError {
                error: "Cache value is not a dataframe".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}

impl CustomValueSupport for NuDataType {
    type CV = NuDataTypeCustomValue;

    fn get_type_static() -> super::PolarsPluginType {
        PolarsPluginType::NuDataType
    }

    fn custom_value(self) -> Self::CV {
        NuDataTypeCustomValue {
            id: self.id,
            datatype: Some(self),
        }
    }

    fn base_value(self, span: Span) -> Result<Value, ShellError> {
        Ok(dtype_to_value(&self.dtype, span))
    }

    fn try_from_value(plugin: &PolarsPlugin, value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::Custom { val, .. } => {
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
            }
            Value::String { val, internal_span } => NuDataType::new_with_str(val, *internal_span),
            _ => Err(ShellError::CantConvert {
                to_type: Self::get_type_static().to_string(),
                from_type: value.get_type().to_string(),
                span: value.span(),
                help: None,
            }),
        }
    }
}

pub fn datatype_list(span: Span) -> Value {
    let types: Vec<Value> = [
        ("null", ""),
        ("bool", ""),
        ("u8", ""),
        ("u16", ""),
        ("u32", ""),
        ("u64", ""),
        ("i8", ""),
        ("i16", ""),
        ("i32", ""),
        ("i64", ""),
        ("f32", ""),
        ("f64", ""),
        ("str", ""),
        ("binary", ""),
        ("date", ""),
        ("datetime<time_unit: (ms, us, ns) timezone (optional)>", "Time Unit can be: milliseconds: ms, microseconds: us, nanoseconds: ns. Timezone wildcard is *. Other Timezone examples: UTC, America/Los_Angeles."),
        ("duration<time_unit: (ms, us, ns)>", "Time Unit can be: milliseconds: ms, microseconds: us, nanoseconds: ns."),
        ("time", ""),
        ("object", ""),
        ("unknown", ""),
        ("list<dtype>", ""),
    ]
    .iter()
    .map(|(dtype, note)| {
        Value::record(record! {
            "dtype" => Value::string(*dtype, span),
            "note" => Value::string(*note, span),
        },
        span)
    })
    .collect();
    Value::list(types, span)
}

pub fn str_to_dtype(dtype: &str, span: Span) -> Result<DataType, ShellError> {
    match dtype {
        "bool" => Ok(DataType::Boolean),
        "u8" => Ok(DataType::UInt8),
        "u16" => Ok(DataType::UInt16),
        "u32" => Ok(DataType::UInt32),
        "u64" => Ok(DataType::UInt64),
        "i8" => Ok(DataType::Int8),
        "i16" => Ok(DataType::Int16),
        "i32" => Ok(DataType::Int32),
        "i64" => Ok(DataType::Int64),
        "f32" => Ok(DataType::Float32),
        "f64" => Ok(DataType::Float64),
        "str" => Ok(DataType::String),
        "binary" => Ok(DataType::Binary),
        "date" => Ok(DataType::Date),
        "time" => Ok(DataType::Time),
        "null" => Ok(DataType::Null),
        "unknown" => Ok(DataType::Unknown(UnknownKind::Any)),
        "object" => Ok(DataType::Object("unknown")),
        _ if dtype.starts_with("list") => {
            let dtype = dtype
                .trim_start_matches("list")
                .trim_start_matches('<')
                .trim_end_matches('>')
                .trim();
            let dtype = str_to_dtype(dtype, span)?;
            Ok(DataType::List(Box::new(dtype)))
        }
        _ if dtype.starts_with("datetime") => {
            let dtype = dtype
                .trim_start_matches("datetime")
                .trim_start_matches('<')
                .trim_end_matches('>');
            let mut split = dtype.split(',');
            let next = split
                .next()
                .ok_or_else(|| ShellError::GenericError {
                    error: "Invalid polars data type".into(),
                    msg: "Missing time unit".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?
                .trim();
            let time_unit = str_to_time_unit(next, span)?;
            let next = split
                .next()
                .ok_or_else(|| ShellError::GenericError {
                    error: "Invalid polars data type".into(),
                    msg: "Missing time zone".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?
                .trim();
            let timezone = if "*" == next {
                None
            } else {
                let zone_str = next.to_string();
                Some(timezone_from_str(&zone_str, None)?)
            };
            Ok(DataType::Datetime(time_unit, timezone))
        }
        _ if dtype.starts_with("duration") => {
            let inner = dtype.trim_start_matches("duration<").trim_end_matches('>');
            let next = inner
                .split(',')
                .next()
                .ok_or_else(|| ShellError::GenericError {
                    error: "Invalid polars data type".into(),
                    msg: "Missing time unit".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?
                .trim();
            let time_unit = str_to_time_unit(next, span)?;
            Ok(DataType::Duration(time_unit))
        }
        _ if dtype.starts_with("decimal") => {
            let dtype = dtype
                .trim_start_matches("decimal")
                .trim_start_matches('<')
                .trim_end_matches('>');
            let mut split = dtype.split(',');
            let next = split
                .next()
                .ok_or_else(|| ShellError::GenericError {
                    error: "Invalid polars data type".into(),
                    msg: "Missing decimal precision".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?
                .trim();
            let precision = match next {
                "*" => None, // infer
                _ => Some(
                    next.parse::<usize>()
                        .map_err(|e| ShellError::GenericError {
                            error: "Invalid polars data type".into(),
                            msg: format!("Error in parsing decimal precision: {e}"),
                            span: Some(span),
                            help: None,
                            inner: vec![],
                        })?,
                ),
            };

            let next = split
                .next()
                .ok_or_else(|| ShellError::GenericError {
                    error: "Invalid polars data type".into(),
                    msg: "Missing decimal scale".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })?
                .trim();
            let scale = match next {
                "*" => Err(ShellError::GenericError {
                    error: "Invalid polars data type".into(),
                    msg: "`*` is not a permitted value for scale".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                }),
                _ => next
                    .parse::<usize>()
                    .map(Some)
                    .map_err(|e| ShellError::GenericError {
                        error: "Invalid polars data type".into(),
                        msg: format!("Error in parsing decimal precision: {e}"),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    }),
            }?;
            Ok(DataType::Decimal(precision, scale))
        }
        _ => Err(ShellError::GenericError {
            error: "Invalid polars data type".into(),
            msg: format!("Unknown type: {dtype}"),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

pub(crate) fn fields_to_value(fields: impl Iterator<Item = Field>, span: Span) -> Value {
    let record = fields
        .map(|field| {
            let col = field.name().to_string();
            let val = dtype_to_value(field.dtype(), span);
            (col, val)
        })
        .collect();

    Value::record(record, Span::unknown())
}

pub fn str_to_time_unit(ts_string: &str, span: Span) -> Result<TimeUnit, ShellError> {
    match ts_string {
        "ms" => Ok(TimeUnit::Milliseconds),
        "us" | "μs" => Ok(TimeUnit::Microseconds),
        "ns" => Ok(TimeUnit::Nanoseconds),
        _ => Err(ShellError::GenericError {
            error: "Invalid polars data type".into(),
            msg: "Invalid time unit".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

pub(crate) fn dtype_to_value(dtype: &DataType, span: Span) -> Value {
    match dtype {
        DataType::Struct(fields) => fields_to_value(fields.iter().cloned(), span),
        DataType::Enum(_, _) => Value::list(
            get_categories(dtype)
                .unwrap_or_default()
                .iter()
                .map(|s| Value::string(s, span))
                .collect(),
            span,
        ),
        _ => Value::string(dtype.to_string().replace('[', "<").replace(']', ">"), span),
    }
}

pub(super) fn get_categories(dtype: &DataType) -> Option<Vec<String>> {
    if let DataType::Enum(frozen_categories, _) = dtype {
        Some(
            frozen_categories
                .categories()
                .iter()
                .filter_map(|v| v.map(ToString::to_string))
                .collect::<Vec<String>>(),
        )
    } else {
        None
    }
}
