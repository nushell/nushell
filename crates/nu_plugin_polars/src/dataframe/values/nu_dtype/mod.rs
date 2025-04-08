mod custom_value;

use custom_value::NuDataTypeCustomValue;
use nu_protocol::{ShellError, Span, Value};
use polars::prelude::DataType;
use uuid::Uuid;

use crate::Cacheable;

use super::{
    nu_schema::dtype_to_value, str_to_dtype, CustomValueSupport, PolarsPluginObject,
    PolarsPluginType,
};

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

impl TryFrom<&Value> for NuDataType {
    type Error = ShellError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::String { val, internal_span } => NuDataType::new_with_str(val, *internal_span),
            _ => Err(ShellError::GenericError {
                error: format!("Unsupported value: {:?}", value),
                msg: "".into(),
                span: Some(value.span()),
                help: None,
                inner: vec![],
            }),
        }
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
}
