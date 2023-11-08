use std::ops::{Deref, DerefMut};

use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime, NaiveTime, Utc};
use indexmap::map::{Entry, IndexMap};
use polars::chunked_array::builder::AnonymousOwnedListBuilder;
use polars::chunked_array::object::builder::ObjectChunkedBuilder;
use polars::chunked_array::ChunkedArray;
use polars::datatypes::AnyValue;
use polars::export::arrow::array::{
    Array, BooleanArray, Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
    UInt16Array, UInt32Array, UInt64Array, UInt8Array,
};
use polars::export::arrow::Either;
use polars::prelude::{
    ArrayRef, DataFrame, DataType, DatetimeChunked, Float64Type, Int64Type, IntoSeries,
    LargeBinaryArray, LargeListArray, LargeStringArray, ListBooleanChunkedBuilder,
    ListBuilderTrait, ListPrimitiveChunkedBuilder, ListType, ListUtf8ChunkedBuilder, NamedFrom,
    NewChunkedArray, ObjectType, Series, StructArray, TemporalMethods, TimeUnit,
};

use nu_protocol::{Record, ShellError, Span, Value};

use super::{DataFrameValue, NuDataFrame};

const SECS_PER_DAY: i64 = 86_400;

// The values capacity is for the size of an  vec.
// Since this is impossible to determine without traversing every value
// I just picked one. Since this is for converting back and forth
// between nushell tables the values shouldn't be too extremely large for
// practical reasons (~ a few thousand rows).
const VALUES_CAPACITY: usize = 10;

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
    Filesize,
    List(Box<InputType>),
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
    span: Span,
) -> Result<Column, ShellError> {
    let size = to_row - from_row;
    let values = series_to_values(series, Some(from_row), Some(size), span)?;
    Ok(Column::new(series.name().into(), values))
}

// Adds a separator to the vector of values using the column names from the
// dataframe to create the Values Row
pub fn add_separator(values: &mut Vec<Value>, df: &DataFrame, span: Span) {
    let mut record = Record::new();

    record.push("index", Value::string("...", span));

    for name in df.get_column_names() {
        record.push(name, Value::string("...", span))
    }

    values.push(Value::record(record, span));
}

// Inserting the values found in a Value::List or Value::Record
pub fn insert_record(column_values: &mut ColumnMap, record: Record) -> Result<(), ShellError> {
    for (col, value) in record {
        insert_value(value, col, column_values)?;
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
        col_val.column_type = Some(value_to_input_type(&value));
        col_val.values.push(value);
    } else {
        let prev_value = &col_val.values[col_val.values.len() - 1];

        match (&prev_value, &value) {
            (Value::Int { .. }, Value::Int { .. })
            | (Value::Float { .. }, Value::Float { .. })
            | (Value::String { .. }, Value::String { .. })
            | (Value::Bool { .. }, Value::Bool { .. })
            | (Value::Date { .. }, Value::Date { .. })
            | (Value::Filesize { .. }, Value::Filesize { .. })
            | (Value::Duration { .. }, Value::Duration { .. }) => col_val.values.push(value),
            (Value::List { .. }, _) => {
                col_val.column_type = Some(value_to_input_type(&value));
                col_val.values.push(value);
            }
            _ => {
                col_val.column_type = Some(InputType::Object);
                col_val.values.push(value);
            }
        }
    }

    Ok(())
}

fn value_to_input_type(value: &Value) -> InputType {
    match &value {
        Value::Int { .. } => InputType::Integer,
        Value::Float { .. } => InputType::Float,
        Value::String { .. } => InputType::String,
        Value::Bool { .. } => InputType::Boolean,
        Value::Date { .. } => InputType::Date,
        Value::Duration { .. } => InputType::Duration,
        Value::Filesize { .. } => InputType::Filesize,
        Value::List { vals, .. } => {
            // We need to determined the type inside of the list.
            // Since Value::List does not have any kind of
            // type information, we need to look inside the list.
            // This will cause errors if lists have inconsistent types.
            // Basically, if a list column needs to be converted to dataframe,
            // needs to have consistent types.
            let list_type = vals
                .iter()
                .filter(|v| !matches!(v, Value::Nothing { .. }))
                .map(value_to_input_type)
                .nth(1)
                .unwrap_or(InputType::Object);

            InputType::List(Box::new(list_type))
        }
        _ => InputType::Object,
    }
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
                InputType::Integer | InputType::Filesize | InputType::Duration => {
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
                    df_series.push(input_type_object_to_series(&name, &column.values)?)
                }
                InputType::List(list_type) => {
                    match input_type_list_to_series(&name, list_type.as_ref(), &column.values) {
                        Ok(series) => df_series.push(series),
                        Err(_) => {
                            // An error case will occur when there are lists of mixed types.
                            // If this happens, fallback to object list
                            df_series.push(input_type_list_to_series(
                                &name,
                                &InputType::Object,
                                &column.values,
                            )?)
                        }
                    }
                }
                InputType::Date => {
                    let it = column.values.iter().map(|v| {
                        if let Value::Date { val, .. } = &v {
                            Some(val.timestamp_nanos_opt().unwrap_or_default())
                        } else {
                            None
                        }
                    });

                    let res: DatetimeChunked =
                        ChunkedArray::<Int64Type>::from_iter_options(&name, it)
                            .into_datetime(TimeUnit::Nanoseconds, None);

                    df_series.push(res.into_series())
                }
            }
        }
    }

    DataFrame::new(df_series)
        .map(|df| NuDataFrame::new(false, df))
        .map_err(|e| {
            ShellError::GenericError(
                "Error creating dataframe".into(),
                "".to_string(),
                None,
                Some(e.to_string()),
                Vec::new(),
            )
        })
}

fn input_type_object_to_series(name: &str, values: &[Value]) -> Result<Series, ShellError> {
    let mut builder = ObjectChunkedBuilder::<DataFrameValue>::new(name, values.len());

    for v in values {
        builder.append_value(DataFrameValue::new(v.clone()));
    }

    let res = builder.finish();
    Ok(res.into_series())
}

fn input_type_list_to_series(
    name: &str,
    list_type: &InputType,
    values: &[Value],
) -> Result<Series, ShellError> {
    let inconsistent_error = |_| {
        ShellError::GenericError(
            format!(
                "column {name} contains a list with inconsistent types: Expecting: {list_type:?}"
            ),
            "".to_string(),
            None,
            None,
            Vec::new(),
        )
    };
    match *list_type {
        // list of boolean values
        InputType::Boolean => {
            let mut builder = ListBooleanChunkedBuilder::new(name, values.len(), VALUES_CAPACITY);
            for v in values {
                let value_list = v
                    .as_list()?
                    .iter()
                    .map(|v| v.as_bool())
                    .collect::<Result<Vec<bool>, _>>()
                    .map_err(inconsistent_error)?;
                builder.append_iter(value_list.iter().map(|v| Some(*v)));
            }
            let res = builder.finish();
            Ok(res.into_series())
        }
        // list of values that reduce down to i64
        InputType::Integer | InputType::Filesize | InputType::Duration => {
            let logical_type = match list_type {
                InputType::Duration => DataType::Duration(TimeUnit::Milliseconds),
                _ => DataType::Int64,
            };

            let mut builder = ListPrimitiveChunkedBuilder::<Int64Type>::new(
                name,
                values.len(),
                VALUES_CAPACITY,
                logical_type,
            );

            for v in values {
                let value_list = v
                    .as_list()?
                    .iter()
                    .map(|v| v.as_i64())
                    .collect::<Result<Vec<i64>, _>>()
                    .map_err(inconsistent_error)?;
                builder.append_iter_values(value_list.iter().copied());
            }
            let res = builder.finish();
            Ok(res.into_series())
        }
        InputType::Float => {
            let mut builder = ListPrimitiveChunkedBuilder::<Float64Type>::new(
                name,
                values.len(),
                VALUES_CAPACITY,
                DataType::Float64,
            );
            for v in values {
                let value_list = v
                    .as_list()?
                    .iter()
                    .map(|v| v.as_f64())
                    .collect::<Result<Vec<f64>, _>>()
                    .map_err(inconsistent_error)?;
                builder.append_iter_values(value_list.iter().copied());
            }
            let res = builder.finish();
            Ok(res.into_series())
        }
        InputType::String => {
            let mut builder = ListUtf8ChunkedBuilder::new(name, values.len(), VALUES_CAPACITY);
            for v in values {
                let value_list = v
                    .as_list()?
                    .iter()
                    .map(|v| v.as_string())
                    .collect::<Result<Vec<String>, _>>()
                    .map_err(inconsistent_error)?;
                builder.append_values_iter(value_list.iter().map(AsRef::as_ref));
            }
            let res = builder.finish();
            Ok(res.into_series())
        }
        // Treat lists as objects at this depth as it is expensive to calculate the list type
        // We can revisit this later if necessary
        InputType::Date => {
            let mut builder = AnonymousOwnedListBuilder::new(
                name,
                values.len(),
                Some(DataType::Datetime(TimeUnit::Nanoseconds, None)),
            );
            for (i, v) in values.iter().enumerate() {
                let list_name = i.to_string();

                let it = v.as_list()?.iter().map(|v| {
                    if let Value::Date { val, .. } = &v {
                        Some(val.timestamp_nanos_opt().unwrap_or_default())
                    } else {
                        None
                    }
                });
                let dt_chunked = ChunkedArray::<Int64Type>::from_iter_options(&list_name, it)
                    .into_datetime(TimeUnit::Nanoseconds, None);

                builder
                    .append_series(&dt_chunked.into_series())
                    .map_err(|e| {
                        ShellError::GenericError(
                            "Error appending to series".into(),
                            "".to_string(),
                            None,
                            Some(e.to_string()),
                            Vec::new(),
                        )
                    })?
            }
            let res = builder.finish();
            Ok(res.into_series())
        }
        InputType::List(ref sub_list_type) => {
            Ok(input_type_list_to_series(name, sub_list_type, values)?)
        }
        // treat everything else as an object
        _ => Ok(input_type_object_to_series(name, values)?),
    }
}

fn series_to_values(
    series: &Series,
    maybe_from_row: Option<usize>,
    maybe_size: Option<usize>,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    match series.dtype() {
        DataType::Null => {
            let it = std::iter::repeat(Value::nothing(span));
            let values = if let Some(size) = maybe_size {
                Either::Left(it.take(size))
            } else {
                Either::Right(it)
            }
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::UInt8 => {
            let casted = series.u8().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to u8".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a as i64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::UInt16 => {
            let casted = series.u16().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to u16".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a as i64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::UInt32 => {
            let casted = series.u32().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to u32".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a as i64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::UInt64 => {
            let casted = series.u64().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to u64".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a as i64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Int8 => {
            let casted = series.i8().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to i8".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a as i64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Int16 => {
            let casted = series.i16().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to i16".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a as i64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Int32 => {
            let casted = series.i32().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to i32".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a as i64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Int64 => {
            let casted = series.i64().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to i64".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::int(a, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Float32 => {
            let casted = series.f32().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to f32".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::float(a as f64, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Float64 => {
            let casted = series.f64().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to f64".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::float(a, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Boolean => {
            let casted = series.bool().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to bool".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::bool(a, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Utf8 => {
            let casted = series.utf8().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to string".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => Value::string(a.to_string(), span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Object(x) => {
            let casted = series
                .as_any()
                .downcast_ref::<ChunkedArray<ObjectType<DataFrameValue>>>();

            match casted {
                None => Err(ShellError::GenericError(
                    "Error casting object from series".into(),
                    "".to_string(),
                    None,
                    Some(format!("Object not supported for conversion: {x}")),
                    Vec::new(),
                )),
                Some(ca) => {
                    let it = ca.into_iter();
                    let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row)
                    {
                        Either::Left(it.skip(from_row).take(size))
                    } else {
                        Either::Right(it)
                    }
                    .map(|v| match v {
                        Some(a) => a.get_value(),
                        None => Value::nothing(span),
                    })
                    .collect::<Vec<Value>>();

                    Ok(values)
                }
            }
        }
        DataType::List(x) => {
            let casted = series.as_any().downcast_ref::<ChunkedArray<ListType>>();
            match casted {
                None => Err(ShellError::GenericError(
                    "Error casting list from series".into(),
                    "".to_string(),
                    None,
                    Some(format!("List not supported for conversion: {x}")),
                    Vec::new(),
                )),
                Some(ca) => {
                    let it = ca.into_iter();
                    if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                        Either::Left(it.skip(from_row).take(size))
                    } else {
                        Either::Right(it)
                    }
                    .map(|ca| {
                        let sublist: Vec<Value> = if let Some(ref s) = ca {
                            series_to_values(s, None, None, Span::unknown())?
                        } else {
                            // empty item
                            vec![]
                        };
                        Ok(Value::list(sublist, span))
                    })
                    .collect::<Result<Vec<Value>, ShellError>>()
                }
            }
        }
        DataType::Date => {
            let casted = series.date().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to date".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => {
                    // elapsed time in day since 1970-01-01
                    let seconds = seconds_per_day(a);
                    let datetime = datetime_from_epoch_seconds(seconds, 0, span)?;
                    Ok(Value::date(datetime, span))
                }
                None => Ok(Value::nothing(span)),
            })
            .collect::<Result<Vec<Value>, ShellError>>()?;
            Ok(values)
        }
        DataType::Datetime(time_unit, _) => {
            let casted = series.datetime().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to datetime".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => {
                    // elapsed time in nano/micro/milliseconds since 1970-01-01
                    let (seconds, nanos) = seconds_and_nanos_from_timeunit(a, *time_unit);
                    let datetime = datetime_from_epoch_seconds(seconds, nanos, span)?;
                    Ok(Value::date(datetime, span))
                }
                None => Ok(Value::nothing(span)),
            })
            .collect::<Result<Vec<Value>, ShellError>>()?;
            Ok(values)
        }
        DataType::Struct(polar_fields) => {
            let casted = series.struct_().map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to struct".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;
            let it = casted.into_iter();
            let values: Result<Vec<Value>, ShellError> =
                if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                    Either::Left(it.skip(from_row).take(size))
                } else {
                    Either::Right(it)
                }
                .map(|any_values| {
                    let vals: Result<Vec<Value>, ShellError> = any_values
                        .iter()
                        .map(|v| any_value_to_value(v, span))
                        .collect();
                    let cols: Vec<String> = polar_fields
                        .iter()
                        .map(|field| field.name.to_string())
                        .collect();
                    let record = Record { cols, vals: vals? };
                    Ok(Value::record(record, span))
                })
                .collect();
            values
        }
        DataType::Time => {
            let casted = series.timestamp(TimeUnit::Nanoseconds).map_err(|e| {
                ShellError::GenericError(
                    "Error casting column to time".into(),
                    "".to_string(),
                    None,
                    Some(e.to_string()),
                    Vec::new(),
                )
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(nanoseconds) => Value::duration(nanoseconds, span),
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        e => Err(ShellError::GenericError(
            "Error creating Dataframe".into(),
            "".to_string(),
            None,
            Some(format!("Value not supported in nushell: {e}")),
            Vec::new(),
        )),
    }
}

fn any_value_to_value(any_value: &AnyValue, span: Span) -> Result<Value, ShellError> {
    match any_value {
        AnyValue::Null => Ok(Value::nothing(span)),
        AnyValue::Boolean(b) => Ok(Value::bool(*b, span)),
        AnyValue::Utf8(s) => Ok(Value::string(s.to_string(), span)),
        AnyValue::UInt8(i) => Ok(Value::int(*i as i64, span)),
        AnyValue::UInt16(i) => Ok(Value::int(*i as i64, span)),
        AnyValue::UInt32(i) => Ok(Value::int(*i as i64, span)),
        AnyValue::UInt64(i) => Ok(Value::int(*i as i64, span)),
        AnyValue::Int8(i) => Ok(Value::int(*i as i64, span)),
        AnyValue::Int16(i) => Ok(Value::int(*i as i64, span)),
        AnyValue::Int32(i) => Ok(Value::int(*i as i64, span)),
        AnyValue::Int64(i) => Ok(Value::int(*i, span)),
        AnyValue::Float32(f) => Ok(Value::float(*f as f64, span)),
        AnyValue::Float64(f) => Ok(Value::float(*f, span)),
        AnyValue::Date(d) => {
            let seconds = seconds_per_day(*d);
            datetime_from_epoch_seconds(seconds, 0, span)
                .map(|datetime| Value::date(datetime, span))
        }
        AnyValue::Datetime(a, time_unit, _) => {
            let (seconds, nanos) = seconds_and_nanos_from_timeunit(*a, *time_unit);
            datetime_from_epoch_seconds(seconds, nanos, span)
                .map(|datetime| Value::date(datetime, span))
        }
        AnyValue::Duration(a, time_unit) => {
            let nanos = match time_unit {
                TimeUnit::Nanoseconds => *a,
                TimeUnit::Microseconds => *a * 1_000,
                TimeUnit::Milliseconds => *a * 1_000_000,
            };
            Ok(Value::duration(nanos, span))
        }
        // AnyValue::Time represents the current time since midnight.
        // Unfortunately, there is no timezone related information.
        // Given this, calculate the current date from UTC and add the time.
        AnyValue::Time(nanos) => time_from_midnight(*nanos, span),
        AnyValue::List(series) => {
            series_to_values(series, None, None, span).map(|values| Value::list(values, span))
        }
        AnyValue::Struct(idx, struct_array, s_fields) => {
            let cols: Vec<String> = s_fields.iter().map(|f| f.name().to_string()).collect();
            let vals: Result<Vec<Value>, ShellError> = struct_array
                .values()
                .iter()
                .map(|v| arr_to_value(&**v, *idx, span))
                .collect();
            let record = Record { cols, vals: vals? };
            Ok(Value::record(record, span))
        }
        AnyValue::StructOwned(struct_tuple) => {
            let values: Result<Vec<Value>, ShellError> = struct_tuple
                .0
                .iter()
                .map(|s| any_value_to_value(s, span))
                .collect();
            let fields = struct_tuple
                .1
                .iter()
                .map(|f| f.name().to_string())
                .collect();
            Ok(Value::Record {
                val: Record {
                    cols: fields,
                    vals: values?,
                },
                internal_span: span,
            })
        }
        AnyValue::Utf8Owned(s) => Ok(Value::string(s.to_string(), span)),
        AnyValue::Binary(bytes) => Ok(Value::binary(*bytes, span)),
        AnyValue::BinaryOwned(bytes) => Ok(Value::binary(bytes.to_owned(), span)),
        e => Err(ShellError::GenericError(
            "Error creating Value".into(),
            "".to_string(),
            None,
            Some(format!("Value not supported in nushell: {e}")),
            Vec::new(),
        )),
    }
}

#[inline]
fn arr_to_value(arr: &dyn Array, idx: usize, span: Span) -> Result<Value, ShellError> {
    macro_rules! downcast {
        ($casttype:ident) => {{
            let arr = &*(arr as *const dyn Array as *const $casttype);
            arr.value_unchecked(idx)
        }};
    }

    let dt = arr.data_type().into();
    // Not loving the unsafe here, however this largely based off the one
    // example I found for converting Array values in:
    // polars_core::chunked_array::ops::any_value::arr_to_any_value
    unsafe {
        match dt {
            DataType::Boolean => Ok(Value::bool(downcast!(BooleanArray), span)),
            DataType::UInt8 => Ok(Value::int(downcast!(UInt8Array) as i64, span)),
            DataType::UInt16 => Ok(Value::int(downcast!(UInt16Array) as i64, span)),
            DataType::UInt32 => Ok(Value::int(downcast!(UInt32Array) as i64, span)),
            DataType::UInt64 => Ok(Value::int(downcast!(UInt64Array) as i64, span)),
            DataType::Int8 => Ok(Value::int(downcast!(Int8Array) as i64, span)),
            DataType::Int16 => Ok(Value::int(downcast!(Int16Array) as i64, span)),
            DataType::Int32 => Ok(Value::int(downcast!(Int32Array) as i64, span)),
            DataType::Int64 => Ok(Value::int(downcast!(Int64Array), span)),
            DataType::Float32 => Ok(Value::float(downcast!(Float32Array) as f64, span)),
            DataType::Float64 => Ok(Value::float(downcast!(Float64Array), span)),
            // DataType::Decimal(_, _) => {}
            DataType::Utf8 => Ok(Value::string(downcast!(LargeStringArray).to_string(), span)),
            DataType::Binary => Ok(Value::binary(downcast!(LargeBinaryArray).to_owned(), span)),
            DataType::Date => {
                let date = downcast!(Int32Array);
                datetime_from_epoch_seconds(date as i64, 0, span)
                    .map(|datetime| Value::date(datetime, span))
            }
            DataType::Datetime(time_unit, _tz) => {
                let (seconds, nanos) =
                    seconds_and_nanos_from_timeunit(downcast!(Int64Array), time_unit);
                datetime_from_epoch_seconds(seconds, nanos, span)
                    .map(|datetime| Value::date(datetime, span))
            }
            // DataType::Duration(_) => {}
            DataType::Time => {
                let t = downcast!(Int64Array);
                time_from_midnight(t, span)
            }
            DataType::List(dt) => {
                let v: ArrayRef = downcast!(LargeListArray);
                let values_result = if dt.is_primitive() {
                    let s = Series::from_chunks_and_dtype_unchecked("", vec![v], &dt);
                    series_to_values(&s, None, None, span)
                } else {
                    let s = Series::from_chunks_and_dtype_unchecked("", vec![v], &dt.to_physical())
                        .cast_unchecked(&dt)
                        .unwrap();
                    series_to_values(&s, None, None, span)
                };
                values_result.map(|values| Value::list(values, span))
            }
            DataType::Null => Ok(Value::nothing(span)),
            DataType::Struct(fields) => {
                let arr = &*(arr as *const dyn Array as *const StructArray);
                let vals: Result<Vec<Value>, ShellError> = arr
                    .values()
                    .iter()
                    .map(|v| arr_to_value(&**v, 0, span))
                    .collect();
                let cols = fields.iter().map(|f| f.name().to_string()).collect();
                Ok(Value::record(Record { cols, vals: vals? }, span))
            }
            DataType::Unknown => Ok(Value::nothing(span)),
            _ => Err(ShellError::CantConvert {
                to_type: dt.to_string(),
                from_type: "polars array".to_string(),
                span,
                help: Some(format!(
                    "Could not convert polars array of type {:?} to value",
                    dt
                )),
            }),
        }
    }
}

fn seconds_per_day(days: i32) -> i64 {
    days as i64 * SECS_PER_DAY
}

fn seconds_and_nanos_from_timeunit(a: i64, time_unit: TimeUnit) -> (i64, u32) {
    // Calculate the divisor for the other units
    let unit_divisor = match time_unit {
        TimeUnit::Nanoseconds => 1_000_000_000,
        TimeUnit::Microseconds => 1_000_000,
        TimeUnit::Milliseconds => 1_000,
    };
    // Calculate seconds and remaining nanoseconds for smaller units
    let seconds = a / unit_divisor;
    let remaining_nanos = a % unit_divisor
        * match time_unit {
            TimeUnit::Microseconds => 1_000, // Convert microseconds to nanoseconds
            TimeUnit::Milliseconds => 1_000_000, // Convert milliseconds to nanoseconds
            TimeUnit::Nanoseconds => 1,      // Already in nanoseconds
        };

    // remaining nanos should fit within u32, and this is the type chrono expects
    (seconds, remaining_nanos as u32)
}

fn datetime_from_epoch_seconds(
    seconds: i64,
    nanos: u32,
    span: Span,
) -> Result<DateTime<FixedOffset>, ShellError> {
    // elapsed time in day since 1970-01-01
    let naive_datetime = match NaiveDateTime::from_timestamp_opt(seconds, nanos) {
        Some(val) => val,
        None => {
            return Err(ShellError::GenericError(
                format!("The given local datetime representation is invalid: seconds: {seconds}, nanos: {nanos}"),
                "".to_string(),
                Some(span),
                None,
                vec![],
            ));
        }
    };
    // Zero length offset
    let offset = match FixedOffset::east_opt(0) {
        Some(val) => val,
        None => {
            // should not happen
            return Err(ShellError::GenericError(
                "Invalid offset: east_opt(0)".to_string(),
                "".to_string(),
                Some(span),
                None,
                vec![],
            ));
        }
    };
    Ok(DateTime::<FixedOffset>::from_naive_utc_and_offset(
        naive_datetime,
        offset,
    ))
}

fn time_from_midnight(nanos: i64, span: Span) -> Result<Value, ShellError> {
    let today = Utc::now().date_naive();
    NaiveTime::from_hms_opt(0, 0, 0) // midnight
        .map(|time| time + Duration::nanoseconds(nanos)) // current time
        .map(|time| today.and_time(time)) // current date and time
        .and_then(|datetime| {
            FixedOffset::east_opt(0) // utc
                .map(|offset| {
                    DateTime::<FixedOffset>::from_naive_utc_and_offset(datetime, offset)
                })
        })
        .map(|datetime| Value::date(datetime, span)) // current date and time
        .ok_or(ShellError::CantConvert {
            to_type: "datetime".to_string(),
            from_type: "polars time".to_string(),
            span,
            help: Some("Could not convert polars time of {nanos} to datetime".to_string()),
        })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use indexmap::indexmap;
    use polars::{export::arrow::array::PrimitiveArray, prelude::Field};

    use super::*;

    #[test]
    fn test_parsed_column_string_list() -> Result<(), Box<dyn std::error::Error>> {
        let values = vec![
            Value::list(
                vec![Value::string("bar".to_string(), Span::test_data())],
                Span::test_data(),
            ),
            Value::list(
                vec![Value::string("baz".to_string(), Span::test_data())],
                Span::test_data(),
            ),
        ];
        let column = Column {
            name: "foo".to_string(),
            values: values.clone(),
        };
        let typed_column = TypedColumn {
            column,
            column_type: Some(InputType::List(Box::new(InputType::String))),
        };

        let column_map = indexmap!("foo".to_string() => typed_column);
        let parsed_df = from_parsed_columns(column_map)?;
        let parsed_columns = parsed_df.columns(Span::test_data())?;
        assert_eq!(parsed_columns.len(), 1);
        let column = parsed_columns
            .first()
            .expect("There should be a first value in columns");
        assert_eq!(column.name(), "foo");
        assert_eq!(column.values, values);

        Ok(())
    }

    #[test]
    fn test_any_value_to_value() -> Result<(), Box<dyn std::error::Error>> {
        let span = Span::test_data();
        assert_eq!(
            any_value_to_value(&AnyValue::Null, span)?,
            Value::nothing(span)
        );

        let test_bool = true;
        assert_eq!(
            any_value_to_value(&AnyValue::Boolean(test_bool), span)?,
            Value::bool(test_bool, span)
        );

        let test_str = "foo";
        assert_eq!(
            any_value_to_value(&AnyValue::Utf8(test_str), span)?,
            Value::string(test_str.to_string(), span)
        );
        assert_eq!(
            any_value_to_value(&AnyValue::Utf8Owned(test_str.into()), span)?,
            Value::string(test_str.to_owned(), span)
        );

        let tests_uint8 = 4;
        assert_eq!(
            any_value_to_value(&AnyValue::UInt8(tests_uint8), span)?,
            Value::int(tests_uint8 as i64, span)
        );

        let tests_uint16 = 233;
        assert_eq!(
            any_value_to_value(&AnyValue::UInt16(tests_uint16), span)?,
            Value::int(tests_uint16 as i64, span)
        );

        let tests_uint32 = 897688233;
        assert_eq!(
            any_value_to_value(&AnyValue::UInt32(tests_uint32), span)?,
            Value::int(tests_uint32 as i64, span)
        );

        let tests_uint64 = 903225135897388233;
        assert_eq!(
            any_value_to_value(&AnyValue::UInt64(tests_uint64), span)?,
            Value::int(tests_uint64 as i64, span)
        );

        let tests_float32 = 903225135897388233.3223353;
        assert_eq!(
            any_value_to_value(&AnyValue::Float32(tests_float32), span)?,
            Value::float(tests_float32 as f64, span)
        );

        let tests_float64 = 9064251358973882322333.64233533232;
        assert_eq!(
            any_value_to_value(&AnyValue::Float64(tests_float64), span)?,
            Value::float(tests_float64 as f64, span)
        );

        let test_days = 10_957;
        let comparison_date = Utc
            .with_ymd_and_hms(2000, 1, 1, 0, 0, 0)
            .unwrap()
            .fixed_offset();
        assert_eq!(
            any_value_to_value(&AnyValue::Date(test_days), span)?,
            Value::date(comparison_date, span)
        );

        let test_millis = 946_684_800_000;
        assert_eq!(
            any_value_to_value(
                &AnyValue::Datetime(test_millis, TimeUnit::Milliseconds, &None),
                span
            )?,
            Value::date(comparison_date, span)
        );

        let test_duration_millis = 99_999;
        let test_duration_micros = 99_999_000;
        let test_duration_nanos = 99_999_000_000;
        assert_eq!(
            any_value_to_value(
                &AnyValue::Duration(test_duration_nanos, TimeUnit::Nanoseconds),
                span
            )?,
            Value::duration(test_duration_nanos, span)
        );
        assert_eq!(
            any_value_to_value(
                &AnyValue::Duration(test_duration_micros, TimeUnit::Microseconds),
                span
            )?,
            Value::duration(test_duration_nanos, span)
        );
        assert_eq!(
            any_value_to_value(
                &AnyValue::Duration(test_duration_millis, TimeUnit::Milliseconds),
                span
            )?,
            Value::duration(test_duration_nanos, span)
        );

        let test_binary = b"sdf2332f32q3f3afwaf3232f32";
        assert_eq!(
            any_value_to_value(&AnyValue::Binary(test_binary), span)?,
            Value::binary(test_binary.to_vec(), span)
        );
        assert_eq!(
            any_value_to_value(&AnyValue::BinaryOwned(test_binary.to_vec()), span)?,
            Value::binary(test_binary.to_vec(), span)
        );

        let test_time_nanos = 54_000_000_000_000;
        let test_time = DateTime::<FixedOffset>::from_naive_utc_and_offset(
            Utc::now()
                .date_naive()
                .and_time(NaiveTime::from_hms_opt(15, 00, 00).unwrap()),
            FixedOffset::east_opt(0).unwrap(),
        );
        assert_eq!(
            any_value_to_value(&AnyValue::Time(test_time_nanos), span)?,
            Value::date(test_time, span)
        );

        let test_list_series = Series::new("int series", &[1, 2, 3]);
        let comparison_list_series = Value::list(
            vec![
                Value::int(1, span),
                Value::int(2, span),
                Value::int(3, span),
            ],
            span,
        );
        assert_eq!(
            any_value_to_value(&AnyValue::List(test_list_series), span)?,
            comparison_list_series
        );

        let field_value_0 = AnyValue::Int32(1);
        let field_value_1 = AnyValue::Boolean(true);
        let values = vec![field_value_0, field_value_1];
        let field_name_0 = "num_field";
        let field_name_1 = "bool_field";
        let fields = vec![
            Field::new(field_name_0, DataType::Int32),
            Field::new(field_name_1, DataType::Boolean),
        ];
        let test_owned_struct = AnyValue::StructOwned(Box::new((values, fields.clone())));
        let comparison_owned_record = Value::record(
            Record {
                cols: vec![field_name_0.to_owned(), field_name_1.to_owned()],
                vals: vec![Value::int(1, span), Value::bool(true, span)],
            },
            span,
        );
        assert_eq!(
            any_value_to_value(&test_owned_struct, span)?,
            comparison_owned_record.clone()
        );

        let test_int_arr = PrimitiveArray::from([Some(1_i32)]);
        let test_bool_arr = BooleanArray::from([Some(true)]);
        let test_struct_arr = StructArray::new(
            DataType::Struct(fields.clone()).to_arrow(),
            vec![Box::new(test_int_arr), Box::new(test_bool_arr)],
            None,
        );
        assert_eq!(
            any_value_to_value(
                &AnyValue::Struct(0, &test_struct_arr, fields.as_slice()),
                span
            )?,
            comparison_owned_record
        );

        Ok(())
    }
}
