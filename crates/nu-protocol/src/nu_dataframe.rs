use std::hash::{Hash, Hasher};
use std::{cmp::Ordering, collections::hash_map::Entry, collections::HashMap};

use nu_errors::ShellError;
use polars::prelude::DataFrame;
use serde::de::{Deserialize, Deserializer, Visitor};
use serde::Serialize;
use std::fmt;

use crate::{Primitive, UntaggedValue, Value};

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
    pub fn try_from_iter<T>(iter: T) -> Result<Self, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        // Dictionary to store the columnar data extracted from
        // the input. During the iteration we will sort if the values
        // have different type
        let mut column_values: HashMap<String, Vec<Value>> = HashMap::new();

        for value in iter {
            match value.value {
                UntaggedValue::Row(dictionary) => {
                    for (key, value) in dictionary.entries {
                        let list = match column_values.entry(key) {
                            Entry::Vacant(entry) => entry.insert(Vec::new()),
                            Entry::Occupied(entry) => entry.into_mut(),
                        };

                        // Checking that the type for this value it the same
                        // for the previous value
                        if list.is_empty() {
                            match &value.value {
                                UntaggedValue::Primitive(_) => {
                                    list.push(value);
                                }
                                _ => {
                                    return Err(ShellError::labeled_error(
                                        "Only primitive values accepted",
                                        "Not a primitive value",
                                        &value.tag,
                                    ));
                                }
                            }
                        } else {
                            let prev_value = &list[list.len() - 1];

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
                                ) => list.push(value),
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
                }
                _ => {
                    return Err(ShellError::labeled_error(
                        "Format not supported",
                        "Value not supported for conversion",
                        &value.tag,
                    ));
                }
            }
        }

        println!("{:?}", column_values);

        Ok(Self::default())
    }
}
