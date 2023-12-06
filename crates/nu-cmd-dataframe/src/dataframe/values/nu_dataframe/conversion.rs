use super::{DataFrameValue, NuDataFrame};

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use indexmap::map::{Entry, IndexMap};
use nu_protocol::{Record, ShellError, Span, Value};
use polars::chunked_array::builder::AnonymousOwnedListBuilder;
use polars::chunked_array::object::builder::ObjectChunkedBuilder;
use polars::chunked_array::ChunkedArray;
use polars::export::arrow::Either;
use polars::prelude::{
    DataFrame, DataType, DatetimeChunked, Float64Type, Int64Type, IntoSeries,
    ListBooleanChunkedBuilder, ListBuilderTrait, ListPrimitiveChunkedBuilder, ListType,
    ListUtf8ChunkedBuilder, NamedFrom, NewChunkedArray, ObjectType, Series, TemporalMethods,
    TimeUnit,
};
use std::ops::{Deref, DerefMut};

const SECS_PER_DAY: i64 = 86_400;

// The values capacity is for the size of an internal vec.
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
            // Since Value::List does not have any kind of internal
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
        .map_err(|e| ShellError::GenericError {
            error: "Error creating dataframe".into(),
            msg: "".into(),
            span: None,
            help: Some(e.to_string()),
            inner: vec![],
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
    let inconsistent_error = |_| ShellError::GenericError {
        error: format!(
            "column {name} contains a list with inconsistent types: Expecting: {list_type:?}"
        ),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
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
                    .map_err(|e| ShellError::GenericError {
                        error: "Error appending to series".into(),
                        msg: "".into(),
                        span: None,
                        help: Some(e.to_string()),
                        inner: vec![],
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
            let casted = series.u8().map_err(|e| ShellError::GenericError {
                error: "Error casting column to u8".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.u16().map_err(|e| ShellError::GenericError {
                error: "Error casting column to u16".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.u32().map_err(|e| ShellError::GenericError {
                error: "Error casting column to u32".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.u64().map_err(|e| ShellError::GenericError {
                error: "Error casting column to u64".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.i8().map_err(|e| ShellError::GenericError {
                error: "Error casting column to i8".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.i16().map_err(|e| ShellError::GenericError {
                error: "Error casting column to i16".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.i32().map_err(|e| ShellError::GenericError {
                error: "Error casting column to i32".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.i64().map_err(|e| ShellError::GenericError {
                error: "Error casting column to i64".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.f32().map_err(|e| ShellError::GenericError {
                error: "Error casting column to f32".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.f64().map_err(|e| ShellError::GenericError {
                error: "Error casting column to f64".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.bool().map_err(|e| ShellError::GenericError {
                error: "Error casting column to bool".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
            let casted = series.utf8().map_err(|e| ShellError::GenericError {
                error: "Error casting column to string".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
                None => Err(ShellError::GenericError {
                    error: "Error casting object from series".into(),
                    msg: "".into(),
                    span: None,
                    help: Some(format!("Object not supported for conversion: {x}")),
                    inner: vec![],
                }),
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
                None => Err(ShellError::GenericError {
                    error: "Error casting list from series".into(),
                    msg: "".into(),
                    span: None,
                    help: Some(format!("List not supported for conversion: {x}")),
                    inner: vec![],
                }),
                Some(ca) => {
                    let it = ca.into_iter();
                    let values: Vec<Value> =
                        if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                            Either::Left(it.skip(from_row).take(size))
                        } else {
                            Either::Right(it)
                        }
                        .map(|ca| {
                            let sublist = ca
                                .map(|ref s| {
                                    match series_to_values(s, None, None, Span::unknown()) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            eprintln!("Error list values: {e}");
                                            vec![]
                                        }
                                    }
                                })
                                .unwrap_or(vec![]);
                            Value::list(sublist, span)
                        })
                        .collect::<Vec<Value>>();
                    Ok(values)
                }
            }
        }
        DataType::Date => {
            let casted = series.date().map_err(|e| ShellError::GenericError {
                error: "Error casting column to date".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
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
                    let seconds = a as i64 * SECS_PER_DAY;
                    let naive_datetime = match NaiveDateTime::from_timestamp_opt(seconds, 0) {
                        Some(val) => val,
                        None => {
                            return Value::error(
                                ShellError::UnsupportedInput {
                                    msg: "The given local datetime representation is invalid."
                                        .to_string(),
                                    input: format!("timestamp is {a:?}"),
                                    msg_span: span,
                                    input_span: Span::unknown(),
                                },
                                span,
                            )
                        }
                    };
                    // Zero length offset
                    let offset = match FixedOffset::east_opt(0) {
                        Some(val) => val,
                        None => {
                            return Value::error(
                                ShellError::UnsupportedInput {
                                    msg: "The given local datetime representation is invalid."
                                        .to_string(),
                                    input: format!("timestamp is {a:?}"),
                                    msg_span: span,
                                    input_span: Span::unknown(),
                                },
                                span,
                            )
                        }
                    };
                    let datetime =
                        DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_datetime, offset);

                    Value::date(datetime, span)
                }
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Datetime(time_unit, _) => {
            let casted = series.datetime().map_err(|e| ShellError::GenericError {
                error: "Error casting column to datetime".into(),
                msg: "".into(),
                span: None,
                help: Some(e.to_string()),
                inner: vec![],
            })?;

            let it = casted.into_iter();
            let values = if let (Some(size), Some(from_row)) = (maybe_size, maybe_from_row) {
                Either::Left(it.skip(from_row).take(size))
            } else {
                Either::Right(it)
            }
            .map(|v| match v {
                Some(a) => {
                    let unit_divisor = match time_unit {
                        TimeUnit::Nanoseconds => 1_000_000_000,
                        TimeUnit::Microseconds => 1_000_000,
                        TimeUnit::Milliseconds => 1_000,
                    };
                    // elapsed time in nano/micro/milliseconds since 1970-01-01
                    let seconds = a / unit_divisor;
                    let naive_datetime = match NaiveDateTime::from_timestamp_opt(seconds, 0) {
                        Some(val) => val,
                        None => {
                            return Value::error(
                                ShellError::UnsupportedInput {
                                    msg: "The given local datetime representation is invalid."
                                        .to_string(),
                                    input: format!("timestamp is {a:?}"),
                                    msg_span: span,
                                    input_span: Span::unknown(),
                                },
                                span,
                            )
                        }
                    };
                    // Zero length offset
                    let offset = match FixedOffset::east_opt(0) {
                        Some(val) => val,
                        None => {
                            return Value::error(
                                ShellError::UnsupportedInput {
                                    msg: "The given local datetime representation is invalid."
                                        .to_string(),
                                    input: format!("timestamp is {a:?}"),
                                    msg_span: span,
                                    input_span: Span::unknown(),
                                },
                                span,
                            )
                        }
                    };
                    let datetime =
                        DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_datetime, offset);

                    Value::date(datetime, span)
                }
                None => Value::nothing(span),
            })
            .collect::<Vec<Value>>();

            Ok(values)
        }
        DataType::Time => {
            let casted =
                series
                    .timestamp(TimeUnit::Nanoseconds)
                    .map_err(|e| ShellError::GenericError {
                        error: "Error casting column to time".into(),
                        msg: "".into(),
                        span: None,
                        help: Some(e.to_string()),
                        inner: vec![],
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
        e => Err(ShellError::GenericError {
            error: "Error creating Dataframe".into(),
            msg: "".to_string(),
            span: None,
            help: Some(format!("Value not supported in nushell: {e}")),
            inner: vec![],
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::indexmap;

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
}
