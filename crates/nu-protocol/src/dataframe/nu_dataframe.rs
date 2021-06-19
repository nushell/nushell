use std::hash::{Hash, Hasher};
use std::{cmp::Ordering, collections::hash_map::Entry, collections::HashMap};

use bigdecimal::FromPrimitive;
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use nu_errors::ShellError;
use nu_source::{Span, Tag};
use num_bigint::BigInt;
use polars::prelude::{AnyValue, DataFrame, NamedFrom, Series, TimeUnit};
use serde::{Deserialize, Serialize};

use crate::{Dictionary, Primitive, UntaggedValue, Value};

use super::PolarsData;

const SECS_PER_DAY: i64 = 86_400;

#[derive(Debug)]
enum InputValue {
    Integer,
    Decimal,
    String,
}

#[derive(Debug)]
struct ColumnValues {
    pub value_type: InputValue,
    pub values: Vec<Value>,
}

impl Default for ColumnValues {
    fn default() -> Self {
        Self {
            value_type: InputValue::Integer,
            values: Vec::new(),
        }
    }
}

type ColumnMap = HashMap<String, ColumnValues>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuDataFrame {
    dataframe: DataFrame,
}

// TODO. Better definition of equality and comparison for a dataframe.
// Probably it make sense to have a name field and use it for comparisons
impl PartialEq for NuDataFrame {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Eq for NuDataFrame {}

impl PartialOrd for NuDataFrame {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl Ord for NuDataFrame {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl Hash for NuDataFrame {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl AsRef<DataFrame> for NuDataFrame {
    fn as_ref(&self) -> &polars::prelude::DataFrame {
        &self.dataframe
    }
}

impl AsMut<DataFrame> for NuDataFrame {
    fn as_mut(&mut self) -> &mut polars::prelude::DataFrame {
        &mut self.dataframe
    }
}

impl NuDataFrame {
    pub fn new(dataframe: polars::prelude::DataFrame) -> Self {
        NuDataFrame { dataframe }
    }

    pub fn try_from_stream<T>(input: &mut T, span: &Span) -> Result<NuDataFrame, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        input
            .next()
            .and_then(|value| match value.value {
                UntaggedValue::DataFrame(PolarsData::EagerDataFrame(df)) => Some(df),
                _ => None,
            })
            .ok_or_else(|| {
                ShellError::labeled_error(
                    "No dataframe in stream",
                    "no dataframe found in input stream",
                    span,
                )
            })
    }

    pub fn try_from_iter<T>(iter: T, tag: &Tag) -> Result<Self, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        // Dictionary to store the columnar data extracted from
        // the input. During the iteration we check if the values
        // have different type
        let mut column_values: ColumnMap = HashMap::new();

        for value in iter {
            match value.value {
                UntaggedValue::Row(dictionary) => insert_row(&mut column_values, dictionary)?,
                UntaggedValue::Table(table) => insert_table(&mut column_values, table)?,
                _ => {
                    return Err(ShellError::labeled_error_with_secondary(
                        "Format not supported",
                        "Value not supported for conversion",
                        &value.tag,
                        "Perhaps you want to use a List of Tables or a Dictionary",
                        &value.tag,
                    ));
                }
            }
        }

        from_parsed_columns(column_values, tag)
    }

    pub fn into_value(self, tag: Tag) -> Value {
        Value {
            value: UntaggedValue::DataFrame(PolarsData::EagerDataFrame(self)),
            tag,
        }
    }

    pub fn dataframe_to_value(df: DataFrame, tag: Tag) -> Value {
        Value {
            value: UntaggedValue::DataFrame(PolarsData::EagerDataFrame(NuDataFrame::new(df))),
            tag,
        }
    }

    // Print is made out a head and if the dataframe is too large, then a tail
    pub fn print(&self) -> Result<Vec<Value>, ShellError> {
        let df = &self.as_ref();
        let size: usize = 20;

        if df.height() > size {
            let sample_size = size / 2;
            let mut values = self.head(Some(sample_size))?;
            add_separator(&mut values, df);
            let remaining = df.height() - sample_size;
            let tail_size = remaining.min(sample_size);
            let mut tail_values = self.tail(Some(tail_size))?;
            values.append(&mut tail_values);

            Ok(values)
        } else {
            Ok(self.head(Some(size))?)
        }
    }

    pub fn head(&self, rows: Option<usize>) -> Result<Vec<Value>, ShellError> {
        let to_row = rows.unwrap_or(5);
        let values = self.to_rows(0, to_row)?;

        Ok(values)
    }

    pub fn tail(&self, rows: Option<usize>) -> Result<Vec<Value>, ShellError> {
        let df = &self.as_ref();
        let to_row = df.height();
        let size = rows.unwrap_or(5);
        let from_row = to_row.saturating_sub(size);

        let values = self.to_rows(from_row, to_row)?;

        Ok(values)
    }

    pub fn to_rows(&self, from_row: usize, to_row: usize) -> Result<Vec<Value>, ShellError> {
        let df = self.as_ref();
        let column_names = df.get_column_names();

        let mut values: Vec<Value> = Vec::new();

        let upper_row = to_row.min(df.height());
        for i in from_row..upper_row {
            let row = df.get_row(i);
            let mut dictionary_row = Dictionary::default();

            for (val, name) in row.0.iter().zip(column_names.iter()) {
                let untagged_val = anyvalue_to_untagged(val)?;

                let dict_val = Value {
                    value: untagged_val,
                    tag: Tag::unknown(),
                };

                dictionary_row.insert(name.to_string(), dict_val);
            }

            let value = Value {
                value: UntaggedValue::Row(dictionary_row),
                tag: Tag::unknown(),
            };

            values.push(value);
        }

        Ok(values)
    }
}

// Adds a separator to the vector of values using the column names from the
// dataframe to create the Values Row
fn add_separator(values: &mut Vec<Value>, df: &DataFrame) {
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

// Converts a polars AnyValue to an UntaggedValue
// This is used when printing values coming for polars dataframes
fn anyvalue_to_untagged(anyvalue: &AnyValue) -> Result<UntaggedValue, ShellError> {
    Ok(match anyvalue {
        AnyValue::Null => UntaggedValue::Primitive(Primitive::Nothing),
        AnyValue::Utf8(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Boolean(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Float32(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Float64(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Int32(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Int64(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::UInt8(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::UInt16(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Int8(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Int16(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::UInt32(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::UInt64(a) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Date32(a) => {
            // elapsed time in day since 1970-01-01
            let seconds = *a as i64 * SECS_PER_DAY;
            let naive_datetime = NaiveDateTime::from_timestamp(seconds, 0);

            // Zero length offset
            let offset = FixedOffset::east(0);
            let datetime = DateTime::<FixedOffset>::from_utc(naive_datetime, offset);

            UntaggedValue::Primitive(Primitive::Date(datetime))
        }
        AnyValue::Date64(a) => {
            // elapsed time in milliseconds since 1970-01-01
            let seconds = *a / 1000;
            let naive_datetime = NaiveDateTime::from_timestamp(seconds, 0);

            // Zero length offset
            let offset = FixedOffset::east(0);
            let datetime = DateTime::<FixedOffset>::from_utc(naive_datetime, offset);

            UntaggedValue::Primitive(Primitive::Date(datetime))
        }
        AnyValue::Time64(a, _) => UntaggedValue::Primitive((*a).into()),
        AnyValue::Duration(a, unit) => {
            let nanoseconds = match unit {
                TimeUnit::Second => *a / 1_000_000_000,
                TimeUnit::Millisecond => *a / 1_000_000,
                TimeUnit::Microsecond => *a / 1_000,
                TimeUnit::Nanosecond => *a,
            };

            if let Some(bigint) = BigInt::from_i64(nanoseconds) {
                UntaggedValue::Primitive(Primitive::Duration(bigint))
            } else {
                unreachable!("Internal error: protocol did not use compatible decimal")
            }
        }
        AnyValue::List(_) => {
            return Err(ShellError::labeled_error(
                "Format not supported",
                "Value not supported for conversion",
                Tag::unknown(),
            ));
        }
    })
}

// Inserting the values found in a UntaggedValue::Row
// All the entries for the dictionary are checked in order to check if
// the column values have the same type value.
fn insert_row(column_values: &mut ColumnMap, dictionary: Dictionary) -> Result<(), ShellError> {
    for (key, value) in dictionary.entries {
        insert_value(value, key, column_values)?;
    }

    Ok(())
}

// Inserting the values found in a UntaggedValue::Table
// All the entries for the table are checked in order to check if
// the column values have the same type value.
// The names for the columns are the enumerated numbers from the values
fn insert_table(column_values: &mut ColumnMap, table: Vec<Value>) -> Result<(), ShellError> {
    for (index, value) in table.into_iter().enumerate() {
        let key = format!("{}", index);
        insert_value(value, key, column_values)?;
    }

    Ok(())
}

fn insert_value(
    value: Value,
    key: String,
    column_values: &mut ColumnMap,
) -> Result<(), ShellError> {
    let col_val = match column_values.entry(key) {
        Entry::Vacant(entry) => entry.insert(ColumnValues::default()),
        Entry::Occupied(entry) => entry.into_mut(),
    };

    // Checking that the type for the value is the same
    // for the previous value in the column
    if col_val.values.is_empty() {
        match &value.value {
            UntaggedValue::Primitive(Primitive::Int(_)) => {
                col_val.value_type = InputValue::Integer;
            }
            UntaggedValue::Primitive(Primitive::Decimal(_)) => {
                col_val.value_type = InputValue::Decimal;
            }
            UntaggedValue::Primitive(Primitive::String(_)) => {
                col_val.value_type = InputValue::String;
            }
            _ => {
                return Err(ShellError::labeled_error(
                    "Only primitive values accepted",
                    "Not a primitive value",
                    &value.tag,
                ));
            }
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
            ) => col_val.values.push(value),
            _ => {
                return Err(ShellError::labeled_error_with_secondary(
                    "Different values in column",
                    "Value with different type",
                    &value.tag,
                    "Perhaps you want to change it to this value type",
                    &prev_value.tag,
                ));
            }
        }
    }

    Ok(())
}

// The ColumnMap has the parsed data from the StreamInput
// This data can be used to create a Series object that can initialize
// the dataframe based on the type of data that is found
fn from_parsed_columns(column_values: ColumnMap, tag: &Tag) -> Result<NuDataFrame, ShellError> {
    let mut df_series: Vec<Series> = Vec::new();
    for (name, column) in column_values {
        match column.value_type {
            InputValue::Decimal => {
                let series_values: Result<Vec<_>, _> =
                    column.values.iter().map(|v| v.as_f64()).collect();
                let series = Series::new(&name, series_values?);
                df_series.push(series)
            }
            InputValue::Integer => {
                let series_values: Result<Vec<_>, _> =
                    column.values.iter().map(|v| v.as_i64()).collect();
                let series = Series::new(&name, series_values?);
                df_series.push(series)
            }
            InputValue::String => {
                let series_values: Result<Vec<_>, _> =
                    column.values.iter().map(|v| v.as_string()).collect();
                let series = Series::new(&name, series_values?);
                df_series.push(series)
            }
        }
    }

    let df = DataFrame::new(df_series);

    match df {
        Ok(df) => Ok(NuDataFrame::new(df)),
        Err(e) => {
            return Err(ShellError::labeled_error(
                "Error while creating dataframe",
                format!("{}", e),
                tag,
            ))
        }
    }
}
