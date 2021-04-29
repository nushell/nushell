use std::hash::{Hash, Hasher};
use std::{cmp::Ordering, collections::hash_map::Entry, collections::HashMap};

use nu_errors::ShellError;
use nu_source::Tag;
use polars::prelude::{DataFrame, NamedFrom, Series};
use serde::de::{Deserialize, Deserializer, Visitor};
use serde::Serialize;

use std::fmt;

use crate::{Dictionary, Primitive, UntaggedValue, Value};

#[derive(Debug)]
enum InputValue {
    Integer,
    Decimal,
    String,
    None,
}

#[derive(Debug)]
struct ColumnValues {
    pub value_type: InputValue,
    pub values: Vec<Value>,
}

impl Default for ColumnValues {
    fn default() -> Self {
        Self {
            value_type: InputValue::None,
            values: Vec::new(),
        }
    }
}

type ColumnMap = HashMap<String, ColumnValues>;

// TODO. Using Option to help with deserialization. It will be better to find
// a way to use serde with dataframes
#[derive(Debug, Clone, Serialize)]
pub struct NuDataFrame {
    #[serde(skip_serializing, default)]
    pub dataframe: Option<DataFrame>,
}

impl Default for NuDataFrame {
    fn default() -> Self {
        NuDataFrame { dataframe: None }
    }
}

impl NuDataFrame {
    fn new() -> Self {
        Self::default()
    }
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

impl<'de> Visitor<'de> for NuDataFrame {
    type Value = Self;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }
}

impl<'de> Deserialize<'de> for NuDataFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_i32(NuDataFrame::new())
    }
}

impl NuDataFrame {
    pub fn try_from_iter<T>(iter: T, tag: &Tag) -> Result<Self, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        // Dictionary to store the columnar data extracted from
        // the input. During the iteration we will sort if the values
        // have different type
        let mut column_values: ColumnMap = HashMap::new();

        for value in iter {
            match value.value {
                UntaggedValue::Row(dictionary) => insert_row(&mut column_values, dictionary)?,
                _ => {
                    return Err(ShellError::labeled_error(
                        "Format not supported",
                        "Value not supported for conversion",
                        &value.tag,
                    ));
                }
            }
        }

        from_parsed_columns(column_values, tag)
    }
}

// Inserting the values found in a UntaggedValue::Row
// All the entries for the dictionary are checked in order to check if
// the column values have the same type value.
fn insert_row(column_values: &mut ColumnMap, dictionary: Dictionary) -> Result<(), ShellError> {
    for (key, value) in dictionary.entries {
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
                    return Err(ShellError::labeled_error(
                        "Different values in column",
                        "Value with different type",
                        &value.tag,
                    ));
                }
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
                    column.values.iter().map(|v| v.as_f32()).collect();
                let series = Series::new(&name, series_values?);
                df_series.push(series)
            }
            InputValue::String => {
                let series_values: Result<Vec<_>, _> =
                    column.values.iter().map(|v| v.as_string()).collect();
                let series = Series::new(&name, series_values?);
                df_series.push(series)
            }
            InputValue::None => {}
        }
    }

    let df = DataFrame::new(df_series);

    println!("{:?}", df);

    match df {
        Ok(df) => Ok(NuDataFrame {
            dataframe: Some(df),
        }),
        Err(e) => {
            return Err(ShellError::labeled_error(
                "Error while creating dataframe",
                format!("{}", e),
                tag,
            ))
        }
    }
}
