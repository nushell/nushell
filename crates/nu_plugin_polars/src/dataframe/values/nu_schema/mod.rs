mod custom_value;

use std::sync::Arc;

use custom_value::NuSchemaCustomValue;
use nu_protocol::{ShellError, Span, Value};
use polars::prelude::{DataType, Field, Schema, SchemaExt, SchemaRef};
use uuid::Uuid;

use crate::Cacheable;

use super::{str_to_dtype, CustomValueSupport, PolarsPluginObject, PolarsPluginType};

#[derive(Debug, Clone)]
pub struct NuSchema {
    pub id: Uuid,
    pub schema: SchemaRef,
}

impl NuSchema {
    pub fn new(schema: SchemaRef) -> Self {
        Self {
            id: Uuid::new_v4(),
            schema,
        }
    }
}

impl TryFrom<&Value> for NuSchema {
    type Error = ShellError;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let schema = value_to_schema(value, Span::unknown())?;
        Ok(Self::new(Arc::new(schema)))
    }
}

impl From<NuSchema> for SchemaRef {
    fn from(val: NuSchema) -> Self {
        Arc::clone(&val.schema)
    }
}

impl From<SchemaRef> for NuSchema {
    fn from(val: SchemaRef) -> Self {
        Self::new(val)
    }
}

impl Cacheable for NuSchema {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<super::PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuSchema(self.clone()))
    }

    fn from_cache_value(cv: super::PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuSchema(dt) => Ok(dt),
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

impl CustomValueSupport for NuSchema {
    type CV = NuSchemaCustomValue;

    fn get_type_static() -> super::PolarsPluginType {
        PolarsPluginType::NuSchema
    }

    fn custom_value(self) -> Self::CV {
        NuSchemaCustomValue {
            id: self.id,
            datatype: Some(self),
        }
    }

    fn base_value(self, span: Span) -> Result<Value, ShellError> {
        Ok(fields_to_value(self.schema.iter_fields(), span))
    }
}

fn fields_to_value(fields: impl Iterator<Item = Field>, span: Span) -> Value {
    let record = fields
        .map(|field| {
            let col = field.name().to_string();
            let val = dtype_to_value(field.dtype(), span);
            (col, val)
        })
        .collect();

    Value::record(record, Span::unknown())
}

pub fn dtype_to_value(dtype: &DataType, span: Span) -> Value {
    match dtype {
        DataType::Struct(fields) => fields_to_value(fields.iter().cloned(), span),
        _ => Value::string(dtype.to_string().replace('[', "<").replace(']', ">"), span),
    }
}

fn value_to_schema(value: &Value, span: Span) -> Result<Schema, ShellError> {
    let fields = value_to_fields(value, span)?;
    let schema = Schema::from_iter(fields);
    Ok(schema)
}

fn value_to_fields(value: &Value, span: Span) -> Result<Vec<Field>, ShellError> {
    let fields = value
        .as_record()?
        .into_iter()
        .map(|(col, val)| match val {
            Value::Record { .. } => {
                let fields = value_to_fields(val, span)?;
                let dtype = DataType::Struct(fields);
                Ok(Field::new(col.into(), dtype))
            }
            _ => {
                let dtype = str_to_dtype(&val.coerce_string()?, span)?;
                Ok(Field::new(col.into(), dtype))
            }
        })
        .collect::<Result<Vec<Field>, ShellError>>()?;
    Ok(fields)
}

#[cfg(test)]
mod test {

    use nu_protocol::record;

    use super::*;

    #[test]
    fn test_value_to_schema() {
        let address = record! {
            "street" => Value::test_string("str"),
            "city" => Value::test_string("str"),
        };

        let value = Value::test_record(record! {
            "name" => Value::test_string("str"),
            "age" => Value::test_string("i32"),
            "address" => Value::test_record(address)
        });

        let schema = value_to_schema(&value, Span::unknown()).unwrap();
        let expected = Schema::from_iter(vec![
            Field::new("name".into(), DataType::String),
            Field::new("age".into(), DataType::Int32),
            Field::new(
                "address".into(),
                DataType::Struct(vec![
                    Field::new("street".into(), DataType::String),
                    Field::new("city".into(), DataType::String),
                ]),
            ),
        ]);
        assert_eq!(schema, expected);
    }
}
