use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use nu_errors::ShellError;
use nu_source::{Span, Tag};
use polars::prelude::{DataType, Series};
use serde::{Deserialize, Serialize};

use crate::{Dictionary, Primitive, UntaggedValue, Value};

use super::PolarsData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuSeries(Series);

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
        NuSeries(series)
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
            .ok_or(ShellError::labeled_error(
                "No series in stream",
                "no series found in input stream",
                span,
            ))
    }

    pub fn series_to_value(series: Series, tag: Tag) -> Value {
        Value {
            value: UntaggedValue::DataFrame(PolarsData::Series(NuSeries::new(series))),
            tag,
        }
    }
}

impl AsRef<Series> for NuSeries {
    fn as_ref(&self) -> &Series {
        &self.0
    }
}

impl AsMut<Series> for NuSeries {
    fn as_mut(&mut self) -> &mut Series {
        &mut self.0
    }
}

macro_rules! series_to_chunked {
    ($converter: expr, $self: expr) => {{
        let chunked_array = $converter.map_err(|e| {
            ShellError::labeled_error("Parsing Error", format!("{}", e), Span::unknown())
        })?;

        let size = 20;

        let (head_size, skip, tail_size) = if $self.0.len() > size {
            let remaining = $self.0.len() - (size / 2);
            let skip = $self.0.len() - remaining;
            (size / 2, skip, remaining.min(size / 2))
        } else {
            (size, 0, 0)
        };

        let head = chunked_array
            .into_iter()
            .take(head_size)
            .map(|value| match value {
                Some(v) => {
                    let mut dictionary_row = Dictionary::default();

                    let value = Value {
                        value: UntaggedValue::Primitive(v.into()),
                        tag: Tag::unknown(),
                    };

                    let header = format!("{} ({})", $self.0.name(), $self.0.dtype());
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

        let res = if $self.0.len() < size {
            head.collect::<Vec<Value>>()
        } else {
            let middle = std::iter::once({
                let mut dictionary_row = Dictionary::default();

                let value = Value {
                    value: UntaggedValue::Primitive("...".into()),
                    tag: Tag::unknown(),
                };

                let header = format!("{} ({})", $self.0.name(), $self.0.dtype());
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

                            let header = format!("{} ({})", $self.0.name(), $self.0.dtype());
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
        match self.0.dtype() {
            DataType::Boolean => series_to_chunked!(self.0.bool(), self),
            DataType::UInt8 => series_to_chunked!(self.0.u8(), self),
            DataType::UInt16 => series_to_chunked!(self.0.u16(), self),
            DataType::UInt32 => series_to_chunked!(self.0.u32(), self),
            DataType::UInt64 => series_to_chunked!(self.0.u64(), self),
            DataType::Int8 => series_to_chunked!(self.0.i8(), self),
            DataType::Int16 => series_to_chunked!(self.0.i16(), self),
            DataType::Int32 => series_to_chunked!(self.0.i32(), self),
            DataType::Int64 => series_to_chunked!(self.0.i64(), self),
            DataType::Float32 => series_to_chunked!(self.0.f32(), self),
            DataType::Float64 => series_to_chunked!(self.0.f64(), self),
            DataType::Utf8 => series_to_chunked!(self.0.utf8(), self),
            DataType::Date32 => series_to_chunked!(self.0.date32(), self),
            DataType::Date64 => series_to_chunked!(self.0.date64(), self),
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
