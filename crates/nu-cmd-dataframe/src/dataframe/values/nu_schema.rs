use nu_protocol::{ShellError, Span, Value, Record};
use polars::prelude::{DataType, Schema, Field};

pub struct NuSchema {
    schema: Schema,
}

impl NuSchema {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }

    pub fn values() -> &'static [&'static str] {
        &[
            "null",
            "bool",
            "u8",
            "u16",
            "u32",
            "u64",
            "i8",
            "i16",
            "i32",
            "i64",
            "f32",
            "f64",
            "decimal[precision, scale]",
            "str",
            "binary",
            "date",
            "datetime[time_unit, timezone]",
            "duration[time_unit]",
            "time",
            "object",
            "unknown",
            "list[dtype]",
        ]
    }
}

impl TryFrom<&Value> for NuSchema {
    type Error = ShellError;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let schema = value_to_schema(value, Span::unknown())?;
        Ok(Self { schema })
    }
}

impl AsRef<Schema> for NuSchema {
    fn as_ref(&self) -> &Schema {
        &self.schema
    }
}

impl Into<Value> for NuSchema {
    fn into(self) -> Value {
        schema_to_value(&self.schema)
    }
}

fn schema_to_value(schema: &Schema) -> Value {

    let (cols, vals) = schema.iter_fields().map(|field| {
        let dtype = Value::string(schema_to_dtype_str(field.data_type()), Span::unknown());
        let col = field.name().to_string();
        (col, dtype)
    })
    .unzip();

    let record = Record::from_raw_cols_vals(cols, vals);
    Value::record(record, Span::unknown())
}

fn schema_to_dtype_str(dtype: &DataType) -> String {
    format!("{}", dtype)
}

fn value_to_schema(value: &Value, span: Span) -> Result<Schema, ShellError> {
    let Record{cols, vals} = value.as_record()?;
    let fields = cols.iter().zip(vals.iter()).map(|(col, val)| {
        let dtype = dtype_str_to_schema(&val.as_string()?, span)?;
        Ok(Field::new(col, dtype))
    }).collect::<Result<Vec<Field>, ShellError>>()?;
    Ok(Schema::from_iter(fields.into_iter()))
}

fn dtype_str_to_schema(dtype: &str, span: Span) -> Result<DataType, ShellError> {
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
        "str" => Ok(DataType::Utf8),
        "binary" => Ok(DataType::Binary),
        "date" => Ok(DataType::Date),
        "time" => Ok(DataType::Time),
        "null" => Ok(DataType::Null),
        "unknown" => Ok(DataType::Unknown),
        _ => Err(ShellError::GenericError {
            error: "Invalid polars data type".into(),
            msg: format!("Unknown type: {dtype}"),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
        // todo support the following
        // "List(Box<DataType>)" => {}
        // "Struct(Vec<Field>)" => {}
        //     "Decimal(Option<usize>, Option<usize>)" => {
        //     }
        //     "Datetime(TimeUnit, Option<TimeZone>)" =>  {
        //     }
        //     "Duration(TimeUnit)" => {}
        //     "Array(Box<DataType>, usize)" => {}
        //     "Object(&'static str)" => {}
    }
}
