use super::NuDataFrame;
use crate::DataFrameValue;
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use indexmap::map::{Entry, IndexMap};
use nu_protocol::{ShellError, Span, Value};
use polars::chunked_array::object::builder::ObjectChunkedBuilder;
use polars::chunked_array::ChunkedArray;
use polars::prelude::{
    DataFrame, DataType, DatetimeChunked, Int64Type, IntoSeries, NamedFrom, NewChunkedArray,
    ObjectType, PolarsNumericType, Series,
};
use std::ops::{Deref, DerefMut};

const SECS_PER_DAY: i64 = 86_400;

#[derive(Debug)]
pub struct Column {
    name: String,
    values: Vec<Value>,
}

impl Column {
    pub fn new(name: String, values: Vec<Value>) -> Self {
        Self { name, values }
    }

    pub fn new_empty(name: String) -> Self {
        Self {
            name,
            values: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.values.iter()
    }
}

impl IntoIterator for Column {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl Deref for Column {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl DerefMut for Column {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

#[derive(Debug)]
pub enum InputType {
    Integer,
    Float,
    String,
    Boolean,
    Object,
    Date,
    Duration,
}

#[derive(Debug)]
pub struct TypedColumn {
    column: Column,
    column_type: Option<InputType>,
}

impl TypedColumn {
    fn new_empty(name: String) -> Self {
        Self {
            column: Column::new_empty(name),
            column_type: None,
        }
    }
}

impl Deref for TypedColumn {
    type Target = Column;

    fn deref(&self) -> &Self::Target {
        &self.column
    }
}

impl DerefMut for TypedColumn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.column
    }
}

pub type ColumnMap = IndexMap<String, TypedColumn>;

pub fn create_column(
    series: &Series,
    from_row: usize,
    to_row: usize,
) -> Result<Column, ShellError> {
    let size = to_row - from_row;
    match series.dtype() {
        DataType::Null => {
            let values = std::iter::repeat(Value::Nothing {
                span: Span::unknown(),
            })
            .take(size)
            .collect::<Vec<Value>>();

            Ok(Column::new(series.name().into(), values))
        }
        DataType::UInt8 => {
            let casted = series
                .u8()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::UInt16 => {
            let casted = series
                .u16()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::UInt32 => {
            let casted = series
                .u32()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::UInt64 => {
            let casted = series
                .u64()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int8 => {
            let casted = series
                .i8()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int16 => {
            let casted = series
                .i16()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int32 => {
            let casted = series
                .i32()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int64 => {
            let casted = series
                .i64()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Float32 => {
            let casted = series
                .f32()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Float64 => {
            let casted = series
                .f64()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Boolean => {
            let casted = series
                .bool()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(a) => Value::Bool {
                        val: a,
                        span: Span::unknown(),
                    },
                    None => Value::Nothing {
                        span: Span::unknown(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Utf8 => {
            let casted = series
                .utf8()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(a) => Value::String {
                        val: a.into(),
                        span: Span::unknown(),
                    },
                    None => Value::Nothing {
                        span: Span::unknown(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Object(x) => {
            let casted = series
                .as_any()
                .downcast_ref::<ChunkedArray<ObjectType<DataFrameValue>>>();

            match casted {
                None => Err(ShellError::InternalError(format!(
                    "Object not supported for conversion: {}",
                    x
                ))),
                Some(ca) => {
                    let values = ca
                        .into_iter()
                        .skip(from_row)
                        .take(size)
                        .map(|v| match v {
                            Some(a) => a.get_value(),
                            None => Value::Nothing {
                                span: Span::unknown(),
                            },
                        })
                        .collect::<Vec<Value>>();

                    Ok(Column::new(ca.name().into(), values))
                }
            }
        }
        DataType::Date => {
            let casted = series
                .date()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(a) => {
                        // elapsed time in day since 1970-01-01
                        let seconds = a as i64 * SECS_PER_DAY;
                        let naive_datetime = NaiveDateTime::from_timestamp(seconds, 0);

                        // Zero length offset
                        let offset = FixedOffset::east(0);
                        let datetime = DateTime::<FixedOffset>::from_utc(naive_datetime, offset);

                        Value::Date {
                            val: datetime,
                            span: Span::unknown(),
                        }
                    }
                    None => Value::Nothing {
                        span: Span::unknown(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Datetime => {
            let casted = series
                .datetime()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(a) => {
                        // elapsed time in milliseconds since 1970-01-01
                        let seconds = a / 1000;
                        let naive_datetime = NaiveDateTime::from_timestamp(seconds, 0);

                        // Zero length offset
                        let offset = FixedOffset::east(0);
                        let datetime = DateTime::<FixedOffset>::from_utc(naive_datetime, offset);

                        Value::Date {
                            val: datetime,
                            span: Span::unknown(),
                        }
                    }
                    None => Value::Nothing {
                        span: Span::unknown(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Time => {
            let casted = series
                .time()
                .map_err(|e| ShellError::InternalError(e.to_string()))?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(nanoseconds) => Value::Duration {
                        val: nanoseconds,
                        span: Span::unknown(),
                    },
                    None => Value::Nothing {
                        span: Span::unknown(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        e => Err(ShellError::InternalError(format!(
            "Value not supported in nushell: {}",
            e
        ))),
    }
}

fn column_from_casted<T>(casted: &ChunkedArray<T>, from_row: usize, size: usize) -> Column
where
    T: PolarsNumericType,
    T::Native: Into<Value>,
{
    let values = casted
        .into_iter()
        .skip(from_row)
        .take(size)
        .map(|v| match v {
            Some(a) => a.into(),
            None => Value::Nothing {
                span: Span::unknown(),
            },
        })
        .collect::<Vec<Value>>();

    Column::new(casted.name().into(), values)
}

// Adds a separator to the vector of values using the column names from the
// dataframe to create the Values Row
pub fn add_separator(values: &mut Vec<Value>, df: &DataFrame) {
    let mut cols = vec![];
    let mut vals = vec![];

    for name in df.get_column_names() {
        cols.push(name.to_string());
        vals.push(Value::String {
            val: "...".into(),
            span: Span::unknown(),
        })
    }

    let extra_record = Value::Record {
        cols,
        vals,
        span: Span::unknown(),
    };

    values.push(extra_record);
}

// Inserting the values found in a Value::List
pub fn insert_record(
    column_values: &mut ColumnMap,
    cols: &[String],
    values: &[Value],
) -> Result<(), ShellError> {
    for (col, value) in cols.iter().zip(values.iter()) {
        insert_value(value.clone(), col.clone(), column_values)?;
    }

    Ok(())
}

pub fn insert_value(
    value: Value,
    key: String,
    column_values: &mut ColumnMap,
) -> Result<(), ShellError> {
    let col_val = match column_values.entry(key.clone()) {
        Entry::Vacant(entry) => entry.insert(TypedColumn::new_empty(key)),
        Entry::Occupied(entry) => entry.into_mut(),
    };

    // Checking that the type for the value is the same
    // for the previous value in the column
    if col_val.values.is_empty() {
        match &value {
            Value::Int { .. } => {
                col_val.column_type = Some(InputType::Integer);
            }
            Value::Float { .. } => {
                col_val.column_type = Some(InputType::Float);
            }
            Value::String { .. } => {
                col_val.column_type = Some(InputType::String);
            }
            Value::Bool { .. } => {
                col_val.column_type = Some(InputType::Boolean);
            }
            Value::Date { .. } => {
                col_val.column_type = Some(InputType::Date);
            }
            Value::Duration { .. } => {
                col_val.column_type = Some(InputType::Duration);
            }
            _ => col_val.column_type = Some(InputType::Object),
        }
        col_val.values.push(value);
    } else {
        let prev_value = &col_val.values[col_val.values.len() - 1];

        match (&prev_value, &value) {
            (Value::Int { .. }, Value::Int { .. })
            | (Value::Float { .. }, Value::Float { .. })
            | (Value::String { .. }, Value::String { .. })
            | (Value::Bool { .. }, Value::Bool { .. })
            | (Value::Date { .. }, Value::Date { .. })
            | (Value::Duration { .. }, Value::Duration { .. }) => col_val.values.push(value),
            _ => {
                col_val.column_type = Some(InputType::Object);
                col_val.values.push(value);
            }
        }
    }

    Ok(())
}

// The ColumnMap has the parsed data from the StreamInput
// This data can be used to create a Series object that can initialize
// the dataframe based on the type of data that is found
pub fn from_parsed_columns(column_values: ColumnMap) -> Result<NuDataFrame, ShellError> {
    let mut df_series: Vec<Series> = Vec::new();
    for (name, column) in column_values {
        if let Some(column_type) = &column.column_type {
            match column_type {
                InputType::Float => {
                    let series_values: Result<Vec<_>, _> =
                        column.values.iter().map(|v| v.as_f64()).collect();
                    let series = Series::new(&name, series_values?);
                    df_series.push(series)
                }
                InputType::Integer => {
                    let series_values: Result<Vec<_>, _> =
                        column.values.iter().map(|v| v.as_i64()).collect();
                    let series = Series::new(&name, series_values?);
                    df_series.push(series)
                }
                InputType::String => {
                    let series_values: Result<Vec<_>, _> =
                        column.values.iter().map(|v| v.as_string()).collect();
                    let series = Series::new(&name, series_values?);
                    df_series.push(series)
                }
                InputType::Boolean => {
                    let series_values: Result<Vec<_>, _> =
                        column.values.iter().map(|v| v.as_bool()).collect();
                    let series = Series::new(&name, series_values?);
                    df_series.push(series)
                }
                InputType::Object => {
                    let mut builder =
                        ObjectChunkedBuilder::<DataFrameValue>::new(&name, column.values.len());

                    for v in &column.values {
                        builder.append_value(DataFrameValue::new(v.clone()));
                    }

                    let res = builder.finish();
                    df_series.push(res.into_series())
                }
                InputType::Date => {
                    let it = column.values.iter().map(|v| {
                        if let Value::Date { val, .. } = &v {
                            Some(val.timestamp_millis())
                        } else {
                            None
                        }
                    });

                    let res: DatetimeChunked =
                        ChunkedArray::<Int64Type>::new_from_opt_iter(&name, it).into();

                    df_series.push(res.into_series())
                }
                InputType::Duration => {
                    let it = column.values.iter().map(|v| {
                        if let Value::Duration { val, .. } = &v {
                            Some(*val)
                        } else {
                            None
                        }
                    });

                    let res = ChunkedArray::<Int64Type>::new_from_opt_iter(&name, it);

                    df_series.push(res.into_series())
                }
            }
        }
    }

    match DataFrame::new(df_series) {
        Ok(df) => Ok(NuDataFrame::new(df)),
        Err(e) => Err(ShellError::InternalError(e.to_string())),
    }
}
