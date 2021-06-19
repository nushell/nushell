use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::vec;

use nu_errors::ShellError;
use nu_source::{Span, Tag};
use polars::prelude::{DataType, NamedFrom, Series};
use serde::{Deserialize, Serialize};

use crate::{Dictionary, Primitive, UntaggedValue, Value};

use super::PolarsData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuSeries {
    series: Series,
    dtype: String,
}

// TODO. Better definition of equality and comparison for a dataframe.
// Probably it make sense to have a name field and use it for comparisons
impl PartialEq for NuSeries {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Eq for NuSeries {}

impl PartialOrd for NuSeries {
    fn partial_cmp(&self, _: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl Ord for NuSeries {
    fn cmp(&self, _: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl Hash for NuSeries {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl NuSeries {
    pub fn new(series: Series) -> Self {
        let dtype = series.dtype().to_string();

        NuSeries { series, dtype }
    }

    pub fn try_from_stream<T>(input: &mut T, span: &Span) -> Result<NuSeries, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        input
            .next()
            .and_then(|value| match value.value {
                UntaggedValue::DataFrame(PolarsData::Series(series)) => Some(series),
                _ => None,
            })
            .ok_or_else(|| {
                ShellError::labeled_error(
                    "No series in stream",
                    "no series found in input stream",
                    span,
                )
            })
    }

    pub fn try_from_iter<T>(iter: T, name: Option<String>) -> Result<Self, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        let mut vec_values: Vec<Value> = Vec::new();

        for value in iter {
            match value.value {
                UntaggedValue::Primitive(Primitive::Int(_))
                | UntaggedValue::Primitive(Primitive::Decimal(_))
                | UntaggedValue::Primitive(Primitive::String(_))
                | UntaggedValue::Primitive(Primitive::Boolean(_)) => {
                    insert_value(value, &mut vec_values)?
                }
                _ => {
                    return Err(ShellError::labeled_error_with_secondary(
                        "Format not supported",
                        "Value not supported for conversion",
                        &value.tag.span,
                        "Perhaps you want to use a list of primitive values (int, decimal, string, or bool)",
                        &value.tag.span,
                    ));
                }
            }
        }

        from_parsed_vector(vec_values, name)
    }

    pub fn into_value(self, tag: Tag) -> Value {
        Value {
            value: UntaggedValue::DataFrame(PolarsData::Series(self)),
            tag,
        }
    }

    pub fn series_to_value(series: Series, tag: Tag) -> Value {
        Value {
            value: UntaggedValue::DataFrame(PolarsData::Series(NuSeries::new(series))),
            tag,
        }
    }

    pub fn series_to_untagged(series: Series) -> UntaggedValue {
        UntaggedValue::DataFrame(PolarsData::Series(NuSeries::new(series)))
    }

    pub fn dtype(&self) -> &str {
        &self.dtype
    }

    pub fn series(self) -> Series {
        self.series
    }
}

impl AsRef<Series> for NuSeries {
    fn as_ref(&self) -> &Series {
        &self.series
    }
}

impl AsMut<Series> for NuSeries {
    fn as_mut(&mut self) -> &mut Series {
        &mut self.series
    }
}

macro_rules! series_to_chunked {
    ($converter: expr, $self: expr) => {{
        let chunked_array = $converter.map_err(|e| {
            ShellError::labeled_error("Parsing Error", format!("{}", e), Span::unknown())
        })?;

        let size = 20;

        let (head_size, skip, tail_size) = if $self.as_ref().len() > size {
            let remaining = $self.as_ref().len() - (size / 2);
            let skip = $self.as_ref().len() - remaining;
            (size / 2, skip, remaining.min(size / 2))
        } else {
            (size, 0, 0)
        };

        let head = chunked_array.into_iter().take(head_size).map(|value| {
            let value = match value {
                Some(v) => Value {
                    value: UntaggedValue::Primitive(v.into()),
                    tag: Tag::unknown(),
                },
                None => Value {
                    value: UntaggedValue::Primitive(Primitive::Nothing),
                    tag: Tag::unknown(),
                },
            };

            let mut dictionary_row = Dictionary::default();
            let header = format!("{} ({})", $self.as_ref().name(), $self.as_ref().dtype());
            dictionary_row.insert(header, value);

            Value {
                value: UntaggedValue::Row(dictionary_row),
                tag: Tag::unknown(),
            }
        });

        let res = if $self.as_ref().len() < size {
            head.collect::<Vec<Value>>()
        } else {
            let middle = std::iter::once({
                let mut dictionary_row = Dictionary::default();

                let value = Value {
                    value: UntaggedValue::Primitive("...".into()),
                    tag: Tag::unknown(),
                };

                let header = format!("{} ({})", $self.as_ref().name(), $self.as_ref().dtype());
                dictionary_row.insert(header, value);

                Value {
                    value: UntaggedValue::Row(dictionary_row),
                    tag: Tag::unknown(),
                }
            });

            let tail =
                chunked_array
                    .into_iter()
                    .skip(skip)
                    .take(tail_size)
                    .map(|value| match value {
                        Some(v) => {
                            let mut dictionary_row = Dictionary::default();

                            let value = Value {
                                value: UntaggedValue::Primitive(v.into()),
                                tag: Tag::unknown(),
                            };

                            let header = format!("{} ({})", $self.as_ref().name(), $self.dtype());
                            dictionary_row.insert(header, value);

                            Value {
                                value: UntaggedValue::Row(dictionary_row),
                                tag: Tag::unknown(),
                            }
                        }
                        None => Value {
                            value: UntaggedValue::Primitive(Primitive::Nothing),
                            tag: Tag::unknown(),
                        },
                    });

            head.chain(middle).chain(tail).collect::<Vec<Value>>()
        };

        Ok(res)
    }};
}

impl NuSeries {
    pub fn print(&self) -> Result<Vec<Value>, ShellError> {
        match self.as_ref().dtype() {
            DataType::Boolean => series_to_chunked!(self.as_ref().bool(), self),
            DataType::UInt8 => series_to_chunked!(self.as_ref().u8(), self),
            DataType::UInt16 => series_to_chunked!(self.as_ref().u16(), self),
            DataType::UInt32 => series_to_chunked!(self.as_ref().u32(), self),
            DataType::UInt64 => series_to_chunked!(self.as_ref().u64(), self),
            DataType::Int8 => series_to_chunked!(self.as_ref().i8(), self),
            DataType::Int16 => series_to_chunked!(self.as_ref().i16(), self),
            DataType::Int32 => series_to_chunked!(self.as_ref().i32(), self),
            DataType::Int64 => series_to_chunked!(self.as_ref().i64(), self),
            DataType::Float32 => series_to_chunked!(self.as_ref().f32(), self),
            DataType::Float64 => series_to_chunked!(self.as_ref().f64(), self),
            DataType::Utf8 => series_to_chunked!(self.as_ref().utf8(), self),
            DataType::Date32 => series_to_chunked!(self.as_ref().date32(), self),
            DataType::Date64 => series_to_chunked!(self.as_ref().date64(), self),
            DataType::Null => Ok(vec![Value {
                value: UntaggedValue::Primitive(Primitive::Nothing),
                tag: Tag::unknown(),
            }]),
            //DataType::List(_) => None,
            //DataType::Time64(TimeUnit) => None,
            //DataType::Duration(TimeUnit) => None,
            //    DataType::Categorical => None,
            _ => unimplemented!(),
        }
    }
}

fn insert_value(value: Value, vec_values: &mut Vec<Value>) -> Result<(), ShellError> {
    // Checking that the type for the value is the same
    // for the previous value in the column
    if vec_values.is_empty() {
        vec_values.push(value);
        Ok(())
    } else {
        let prev_value = &vec_values[vec_values.len() - 1];

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
            ) => {
                vec_values.push(value);
                Ok(())
            }
            _ => Err(ShellError::labeled_error_with_secondary(
                "Different values in column",
                "Value with different type",
                &value.tag,
                "Perhaps you want to change it to this value type",
                &prev_value.tag,
            )),
        }
    }
}

fn from_parsed_vector(
    vec_values: Vec<Value>,
    name: Option<String>,
) -> Result<NuSeries, ShellError> {
    let series = match &vec_values[0].value {
        UntaggedValue::Primitive(Primitive::Int(_)) => {
            let series_values: Result<Vec<_>, _> = vec_values.iter().map(|v| v.as_i64()).collect();
            let series_name = match &name {
                Some(n) => n.as_ref(),
                None => "int",
            };
            Series::new(series_name, series_values?)
        }
        UntaggedValue::Primitive(Primitive::Decimal(_)) => {
            let series_values: Result<Vec<_>, _> = vec_values.iter().map(|v| v.as_f64()).collect();
            let series_name = match &name {
                Some(n) => n.as_ref(),
                None => "decimal",
            };
            Series::new(series_name, series_values?)
        }
        UntaggedValue::Primitive(Primitive::String(_)) => {
            let series_values: Result<Vec<_>, _> =
                vec_values.iter().map(|v| v.as_string()).collect();
            let series_name = match &name {
                Some(n) => n.as_ref(),
                None => "string",
            };
            Series::new(series_name, series_values?)
        }
        UntaggedValue::Primitive(Primitive::Boolean(_)) => {
            let series_values: Result<Vec<_>, _> = vec_values.iter().map(|v| v.as_bool()).collect();
            let series_name = match &name {
                Some(n) => n.as_ref(),
                None => "string",
            };
            Series::new(series_name, series_values?)
        }
        _ => unreachable!("The untagged type is checked while creating vec_values"),
    };

    Ok(NuSeries::new(series))
}
