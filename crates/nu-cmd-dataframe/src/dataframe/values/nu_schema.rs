use std::sync::Arc;

use nu_protocol::{Record, ShellError, Span, Value};
use polars::prelude::{DataType, Field, Schema, SchemaRef, TimeUnit};

pub struct NuSchema {
    schema: SchemaRef,
}

impl NuSchema {
    pub fn new(schema: Schema) -> Self {
        Self {
            schema: Arc::new(schema),
        }
    }
}

impl TryFrom<&Value> for NuSchema {
    type Error = ShellError;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let schema = value_to_schema(value, Span::unknown())?;
        Ok(Self::new(schema))
    }
}

impl From<NuSchema> for Value {
    fn from(val: NuSchema) -> Self {
        schema_to_value(&val.schema)
    }
}

impl From<NuSchema> for SchemaRef {
    fn from(val: NuSchema) -> Self {
        Arc::clone(&val.schema)
    }
}

fn schema_to_value(schema: &Schema) -> Value {
    let (cols, vals) = schema
        .iter_fields()
        .map(|field| {
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
    let fields = value_to_fields(value, span)?;
    let schema = Schema::from_iter(fields);
    Ok(schema)
}

fn value_to_fields(value: &Value, span: Span) -> Result<Vec<Field>, ShellError> {
    let Record { cols, vals } = value.as_record()?;
    let fields = cols
        .iter()
        .zip(vals.iter())
        .map(|(col, val)| match val {
            Value::Record { .. } => {
                let fields = value_to_fields(val, span)?;
                let dtype = DataType::Struct(fields);
                Ok(Field::new(col, dtype))
            }
            _ => {
                let dtype = dtype_str_to_schema(&val.as_string()?, span)?;
                Ok(Field::new(col, dtype))
            }
        })
        .collect::<Result<Vec<Field>, ShellError>>()?;
    Ok(fields)
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
        "object" => Ok(DataType::Object("unknown")),
        _ if dtype.starts_with("list") => {
            let dtype = dtype
                .trim_start_matches("list")
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim();
            let dtype = dtype_str_to_schema(dtype, span)?;
            Ok(DataType::List(Box::new(dtype)))
        }
        _ if dtype.starts_with("datetime") => {
            let dtype = dtype
                .trim_start_matches("datetime")
                .trim_start_matches('[')
                .trim_end_matches(']');
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
                Some(next.to_string())
            };
            Ok(DataType::Datetime(time_unit, timezone))
        }
        _ if dtype.starts_with("duration") => {
            let inner = dtype.trim_start_matches("duration[").trim_end_matches(']');
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
        _ => Err(ShellError::GenericError {
            error: "Invalid polars data type".into(),
            msg: format!("Unknown type: {dtype}"),
            span: Some(span),
            help: None,
            inner: vec![],
        }),
    }
}

fn str_to_time_unit(ts_string: &str, span: Span) -> Result<TimeUnit, ShellError> {
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

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_value_to_schema() {
        let value = Value::Record {
            val: Record {
                cols: vec!["name".into(), "age".into(), "address".into()],
                vals: vec![
                    Value::String {
                        val: "str".into(),
                        internal_span: Span::test_data(),
                    },
                    Value::String {
                        val: "i32".into(),
                        internal_span: Span::test_data(),
                    },
                    Value::Record {
                        val: Record {
                            cols: vec!["street".into(), "city".into()],
                            vals: vec![
                                Value::String {
                                    val: "str".into(),
                                    internal_span: Span::test_data(),
                                },
                                Value::String {
                                    val: "str".into(),
                                    internal_span: Span::test_data(),
                                },
                            ],
                        },
                        internal_span: Span::test_data(),
                    },
                ],
            },
            internal_span: Span::test_data(),
        };
        let schema = value_to_schema(&value, Span::unknown()).unwrap();
        let expected = Schema::from_iter(vec![
            Field::new("name", DataType::Utf8),
            Field::new("age", DataType::Int32),
            Field::new(
                "address",
                DataType::Struct(vec![
                    Field::new("street", DataType::Utf8),
                    Field::new("city", DataType::Utf8),
                ]),
            ),
        ]);
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_dtype_str_to_schema_simple_types() {
        let dtype = "bool";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Boolean;
        assert_eq!(schema, expected);

        let dtype = "u8";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt8;
        assert_eq!(schema, expected);

        let dtype = "u16";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt16;
        assert_eq!(schema, expected);

        let dtype = "u32";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt32;
        assert_eq!(schema, expected);

        let dtype = "u64";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::UInt64;
        assert_eq!(schema, expected);

        let dtype = "i8";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int8;
        assert_eq!(schema, expected);

        let dtype = "i16";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int16;
        assert_eq!(schema, expected);

        let dtype = "i32";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int32;
        assert_eq!(schema, expected);

        let dtype = "i64";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Int64;
        assert_eq!(schema, expected);

        let dtype = "str";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Utf8;
        assert_eq!(schema, expected);

        let dtype = "binary";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Binary;
        assert_eq!(schema, expected);

        let dtype = "date";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Date;
        assert_eq!(schema, expected);

        let dtype = "time";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Time;
        assert_eq!(schema, expected);

        let dtype = "null";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Null;
        assert_eq!(schema, expected);

        let dtype = "unknown";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Unknown;
        assert_eq!(schema, expected);

        let dtype = "object";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Object("unknown");
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_dtype_str_schema_datetime() {
        let dtype = "datetime[ms, *]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Milliseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime[us, *]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Microseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime[μs, *]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Microseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime[ns, *]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Nanoseconds, None);
        assert_eq!(schema, expected);

        let dtype = "datetime[ms, UTC]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Datetime(TimeUnit::Milliseconds, Some("UTC".into()));
        assert_eq!(schema, expected);

        let dtype = "invalid";
        let schema = dtype_str_to_schema(dtype, Span::unknown());
        assert!(schema.is_err())
    }

    #[test]
    fn test_dtype_str_schema_duration() {
        let dtype = "duration[ms]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Milliseconds);
        assert_eq!(schema, expected);

        let dtype = "duration[us]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Microseconds);
        assert_eq!(schema, expected);

        let dtype = "duration[μs]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Microseconds);
        assert_eq!(schema, expected);

        let dtype = "duration[ns]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::Duration(TimeUnit::Nanoseconds);
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_dtype_str_to_schema_list_types() {
        let dtype = "list[i32]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Int32));
        assert_eq!(schema, expected);

        let dtype = "list[duration[ms]]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Duration(TimeUnit::Milliseconds)));
        assert_eq!(schema, expected);

        let dtype = "list[datetime[ms, *]]";
        let schema = dtype_str_to_schema(dtype, Span::unknown()).unwrap();
        let expected = DataType::List(Box::new(DataType::Datetime(TimeUnit::Milliseconds, None)));
        assert_eq!(schema, expected);
    }
}
