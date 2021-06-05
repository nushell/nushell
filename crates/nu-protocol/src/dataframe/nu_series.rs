use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use nu_errors::ShellError;
use nu_source::{Span, Tag};
use polars::prelude::{DataType, Series};
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

    pub fn series_to_untagged(series: Series) -> UntaggedValue {
        UntaggedValue::DataFrame(PolarsData::Series(NuSeries::new(series)))
    }

    pub fn dtype(&self) -> &str {
        &self.dtype
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

                    let header = format!("{} ({})", $self.as_ref().name(), $self.as_ref().dtype());
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
