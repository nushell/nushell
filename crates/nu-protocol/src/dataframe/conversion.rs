use indexmap::map::{Entry, IndexMap};
use polars::chunked_array::object::builder::ObjectChunkedBuilder;
use polars::chunked_array::ChunkedArray;

use bigdecimal::{FromPrimitive, ToPrimitive};
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use nu_errors::ShellError;
use nu_source::{Span, Tag};
use num_bigint::BigInt;
use polars::prelude::{
    DataFrame, DataType, Date64Type, Int64Type, IntoSeries, NamedFrom, NewChunkedArray, ObjectType,
    PolarsNumericType, Series, TimeUnit,
};
use std::ops::{Deref, DerefMut};

use super::NuDataFrame;
use crate::{Dictionary, Primitive, UntaggedValue, Value};

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
    Decimal,
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
            let values = std::iter::repeat(Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                tag: Tag::default(),
            })
            .take(size)
            .collect::<Vec<Value>>();

            Ok(Column::new(series.name().into(), values))
        }
        DataType::UInt8 => {
            let casted = series.u8().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::UInt16 => {
            let casted = series.u16().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::UInt32 => {
            let casted = series.u32().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::UInt64 => {
            let casted = series.u64().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int8 => {
            let casted = series.i8().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int16 => {
            let casted = series.i16().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int32 => {
            let casted = series.i32().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Int64 => {
            let casted = series.i64().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Float32 => {
            let casted = series.f32().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Float64 => {
            let casted = series.f64().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;
            Ok(column_from_casted(casted, from_row, size))
        }
        DataType::Boolean => {
            let casted = series.bool().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(a) => Value {
                        value: UntaggedValue::Primitive((a).into()),
                        tag: Tag::default(),
                    },
                    None => Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        tag: Tag::default(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Utf8 => {
            let casted = series.utf8().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(a) => Value {
                        value: UntaggedValue::Primitive((a).into()),
                        tag: Tag::default(),
                    },
                    None => Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        tag: Tag::default(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Object(_) => {
            let casted = series
                .as_any()
                .downcast_ref::<ChunkedArray<ObjectType<Value>>>();

            match casted {
                None => Err(ShellError::labeled_error(
                    "Format not supported",
                    "Value not supported for conversion",
                    Tag::unknown(),
                )),
                Some(ca) => {
                    let values = ca
                        .into_iter()
                        .skip(from_row)
                        .take(size)
                        .map(|v| match v {
                            Some(a) => a.clone(),
                            None => Value {
                                value: UntaggedValue::Primitive(Primitive::Nothing),
                                tag: Tag::default(),
                            },
                        })
                        .collect::<Vec<Value>>();

                    Ok(Column::new(ca.name().into(), values))
                }
            }
        }
        DataType::Date32 => {
            let casted = series.date32().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;

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

                        Value {
                            value: UntaggedValue::Primitive(Primitive::Date(datetime)),
                            tag: Tag::default(),
                        }
                    }
                    None => Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        tag: Tag::default(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Date64 => {
            let casted = series.date64().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;

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

                        Value {
                            value: UntaggedValue::Primitive(Primitive::Date(datetime)),
                            tag: Tag::default(),
                        }
                    }
                    None => Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        tag: Tag::default(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        DataType::Time64(timeunit) | DataType::Duration(timeunit) => {
            let casted = series.time64_nanosecond().map_err(|e| {
                ShellError::labeled_error(
                    "Casting error",
                    format!("casting error: {}", e),
                    Span::default(),
                )
            })?;

            let values = casted
                .into_iter()
                .skip(from_row)
                .take(size)
                .map(|v| match v {
                    Some(a) => {
                        let nanoseconds = match timeunit {
                            TimeUnit::Second => a / 1_000_000_000,
                            TimeUnit::Millisecond => a / 1_000_000,
                            TimeUnit::Microsecond => a / 1_000,
                            TimeUnit::Nanosecond => a,
                        };

                        let untagged = if let Some(bigint) = BigInt::from_i64(nanoseconds) {
                            UntaggedValue::Primitive(Primitive::Duration(bigint))
                        } else {
                            unreachable!("Internal error: protocol did not use compatible decimal")
                        };

                        Value {
                            value: untagged,
                            tag: Tag::default(),
                        }
                    }
                    None => Value {
                        value: UntaggedValue::Primitive(Primitive::Nothing),
                        tag: Tag::default(),
                    },
                })
                .collect::<Vec<Value>>();

            Ok(Column::new(casted.name().into(), values))
        }
        e => Err(ShellError::labeled_error(
            "Format not supported",
            format!("Value not supported for conversion: {}", e),
            Tag::unknown(),
        )),
    }
}

fn column_from_casted<T>(casted: &ChunkedArray<T>, from_row: usize, size: usize) -> Column
where
    T: PolarsNumericType,
    T::Native: Into<Primitive>,
{
    let values = casted
        .into_iter()
        .skip(from_row)
        .take(size)
        .map(|v| match v {
            Some(a) => Value {
                value: UntaggedValue::Primitive((a).into()),
                tag: Tag::default(),
            },
            None => Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                tag: Tag::default(),
            },
        })
        .collect::<Vec<Value>>();

    Column::new(casted.name().into(), values)
}

// Adds a separator to the vector of values using the column names from the
// dataframe to create the Values Row
pub fn add_separator(values: &mut Vec<Value>, df: &DataFrame) {
    let column_names = df.get_column_names();

    let mut dictionary = Dictionary::default();
    for name in column_names {
        let indicator = Value {
            value: UntaggedValue::Primitive(Primitive::String("...".to_string())),
            tag: Tag::unknown(),
        };

        dictionary.insert(name.to_string(), indicator);
    }

    let extra_column = Value {
        value: UntaggedValue::Row(dictionary),
        tag: Tag::unknown(),
    };

    values.push(extra_column);
}

// Inserting the values found in a UntaggedValue::Row
// All the entries for the dictionary are checked in order to check if
// the column values have the same type value.
pub fn insert_row(column_values: &mut ColumnMap, dictionary: Dictionary) -> Result<(), ShellError> {
    for (key, value) in dictionary.entries {
        insert_value(value, key, column_values)?;
    }

    Ok(())
}

// Inserting the values found in a UntaggedValue::Table
// All the entries for the table are checked in order to check if
// the column values have the same type value.
// The names for the columns are the enumerated numbers from the values
pub fn insert_table(column_values: &mut ColumnMap, table: Vec<Value>) -> Result<(), ShellError> {
    for (index, value) in table.into_iter().enumerate() {
        let key = index.to_string();
        insert_value(value, key, column_values)?;
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
        match &value.value {
            UntaggedValue::Primitive(Primitive::Int(_)) => {
                col_val.column_type = Some(InputType::Integer);
            }
            UntaggedValue::Primitive(Primitive::Decimal(_)) => {
                col_val.column_type = Some(InputType::Decimal);
            }
            UntaggedValue::Primitive(Primitive::String(_)) => {
                col_val.column_type = Some(InputType::String);
            }
            UntaggedValue::Primitive(Primitive::Boolean(_)) => {
                col_val.column_type = Some(InputType::Boolean);
            }
            UntaggedValue::Primitive(Primitive::Date(_)) => {
                col_val.column_type = Some(InputType::Date);
            }
            UntaggedValue::Primitive(Primitive::Duration(_)) => {
                col_val.column_type = Some(InputType::Duration);
            }
            _ => col_val.column_type = Some(InputType::Object),
        }
        col_val.values.push(value);
    } else {
        let prev_value = &col_val.values[col_val.values.len() - 1];

        match (&prev_value.value, &value.value) {
            (
                UntaggedValue::Primitive(Primitive::Int(_)),
                UntaggedValue::Primitive(Primitive::Int(_)),
            )
            | (
                UntaggedValue::Primitive(Primitive::Decimal(_)),
                UntaggedValue::Primitive(Primitive::Decimal(_)),
            )
            | (
                UntaggedValue::Primitive(Primitive::String(_)),
                UntaggedValue::Primitive(Primitive::String(_)),
            )
            | (
                UntaggedValue::Primitive(Primitive::Boolean(_)),
                UntaggedValue::Primitive(Primitive::Boolean(_)),
            )
            | (
                UntaggedValue::Primitive(Primitive::Date(_)),
                UntaggedValue::Primitive(Primitive::Date(_)),
            )
            | (
                UntaggedValue::Primitive(Primitive::Duration(_)),
                UntaggedValue::Primitive(Primitive::Duration(_)),
            ) => col_val.values.push(value),
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
pub fn from_parsed_columns(
    column_values: ColumnMap,
    span: &Span,
) -> Result<NuDataFrame, ShellError> {
    let mut df_series: Vec<Series> = Vec::new();
    for (name, column) in column_values {
        if let Some(column_type) = &column.column_type {
            match column_type {
                InputType::Decimal => {
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
                        ObjectChunkedBuilder::<Value>::new(&name, column.values.len());

                    for v in &column.values {
                        builder.append_value(v.clone());
                    }

                    let res = builder.finish();
                    df_series.push(res.into_series())
                }
                InputType::Date => {
                    let it = column.values.iter().map(|v| {
                        if let UntaggedValue::Primitive(Primitive::Date(date)) = &v.value {
                            Some(date.timestamp_millis())
                        } else {
                            None
                        }
                    });

                    let res = ChunkedArray::<Date64Type>::new_from_opt_iter(&name, it);

                    df_series.push(res.into_series())
                }
                InputType::Duration => {
                    let it = column.values.iter().map(|v| {
                        if let UntaggedValue::Primitive(Primitive::Duration(duration)) = &v.value {
                            Some(duration.to_i64().expect("Not expecting NAN in duration"))
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

    let df = DataFrame::new(df_series);

    match df {
        Ok(df) => Ok(NuDataFrame::new(df)),
        Err(e) => Err(ShellError::labeled_error(
            "Error while creating dataframe",
            e.to_string(),
            span,
        )),
    }
}
