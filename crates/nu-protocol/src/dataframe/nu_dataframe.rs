use indexmap::IndexMap;
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

use nu_errors::ShellError;
use nu_source::{Span, Tag};
use polars::prelude::{DataFrame, DataType, PolarsObject, Series};
use serde::{Deserialize, Serialize};

use super::conversion::{
    add_separator, create_column, from_parsed_columns, insert_row, insert_table, insert_value,
    Column, ColumnMap,
};
use crate::{Dictionary, Primitive, ShellTypeName, UntaggedValue, Value};

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

impl Default for Value {
    fn default() -> Self {
        Self {
            value: UntaggedValue::Primitive(Primitive::Nothing),
            tag: Tag::default(),
        }
    }
}

impl PolarsObject for Value {
    fn type_name() -> &'static str {
        "object"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuDataFrame {
    dataframe: DataFrame,
}

// Dataframes are considered equal if they have the same shape, column name
// and values
impl PartialEq for NuDataFrame {
    fn eq(&self, other: &Self) -> bool {
        if self.as_ref().width() == 0 {
            // checking for empty dataframe
            return false;
        }

        if self.as_ref().get_column_names() != other.as_ref().get_column_names() {
            // checking both dataframes share the same names
            return false;
        }

        if self.as_ref().height() != other.as_ref().height() {
            // checking both dataframes have the same row size
            return false;
        }

        // sorting dataframe by the first column
        let column_names = self.as_ref().get_column_names();
        let first_col = column_names
            .get(0)
            .expect("already checked that dataframe is different than 0");

        // if unable to sort, then unable to compare
        let lhs = match self.as_ref().sort(*first_col, false) {
            Ok(df) => df,
            Err(_) => return false,
        };

        let rhs = match other.as_ref().sort(*first_col, false) {
            Ok(df) => df,
            Err(_) => return false,
        };

        for name in self.as_ref().get_column_names() {
            let self_series = lhs.column(name).expect("name from dataframe names");

            let other_series = rhs
                .column(name)
                .expect("already checked that name in other");

            let self_series = match self_series.dtype() {
                // Casting needed to compare other numeric types with nushell numeric type.
                // In nushell we only have i64 integer numeric types and any array created
                // with nushell untagged primitives will be of type i64
                DataType::UInt32 => match self_series.cast_with_dtype(&DataType::Int64) {
                    Ok(series) => series,
                    Err(_) => return false,
                },
                _ => self_series.clone(),
            };

            if !self_series.series_equal(other_series) {
                return false;
            }
        }

        true
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

    pub fn try_from_stream<T>(input: &mut T, span: &Span) -> Result<(Self, Tag), ShellError>
    where
        T: Iterator<Item = Value>,
    {
        input
            .next()
            .and_then(|value| match value.value {
                UntaggedValue::DataFrame(df) => Some((df, value.tag)),
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
        let mut column_values: ColumnMap = IndexMap::new();

        for value in iter {
            match value.value {
                UntaggedValue::Row(dictionary) => insert_row(&mut column_values, dictionary)?,
                UntaggedValue::Table(table) => insert_table(&mut column_values, table)?,
                UntaggedValue::Primitive(Primitive::Int(_))
                | UntaggedValue::Primitive(Primitive::Decimal(_))
                | UntaggedValue::Primitive(Primitive::String(_))
                | UntaggedValue::Primitive(Primitive::Boolean(_))
                | UntaggedValue::Primitive(Primitive::Date(_))
                | UntaggedValue::DataFrame(_) => {
                    let key = "0".to_string();
                    insert_value(value, key, &mut column_values)?
                }
                _ => {
                    return Err(ShellError::labeled_error_with_secondary(
                        "Format not supported",
                        "Value not supported for conversion",
                        &value.tag,
                        "Perhaps you want to use a List, a List of Tables or a Dictionary",
                        &value.tag,
                    ));
                }
            }
        }

        from_parsed_columns(column_values, &tag.span)
    }

    pub fn try_from_series(columns: Vec<Series>, span: &Span) -> Result<Self, ShellError> {
        let dataframe = DataFrame::new(columns).map_err(|e| {
            ShellError::labeled_error(
                "DataFrame Creation",
                format!("Unable to create DataFrame: {}", e),
                span,
            )
        })?;

        Ok(Self { dataframe })
    }

    pub fn try_from_columns(columns: Vec<Column>, span: &Span) -> Result<Self, ShellError> {
        let mut column_values: ColumnMap = IndexMap::new();

        for column in columns {
            let name = column.name().to_string();
            for value in column {
                insert_value(value, name.clone(), &mut column_values)?;
            }
        }

        from_parsed_columns(column_values, span)
    }

    pub fn into_value(self, tag: Tag) -> Value {
        Value {
            value: Self::into_untagged(self),
            tag,
        }
    }

    pub fn into_untagged(self) -> UntaggedValue {
        UntaggedValue::DataFrame(self)
    }

    pub fn dataframe_to_value(df: DataFrame, tag: Tag) -> Value {
        Value {
            value: Self::dataframe_to_untagged(df),
            tag,
        }
    }

    pub fn dataframe_to_untagged(df: DataFrame) -> UntaggedValue {
        UntaggedValue::DataFrame(Self::new(df))
    }

    pub fn series_to_untagged(series: Series, span: &Span) -> UntaggedValue {
        match DataFrame::new(vec![series]) {
            Ok(dataframe) => UntaggedValue::DataFrame(Self { dataframe }),
            Err(e) => UntaggedValue::Error(ShellError::labeled_error(
                "DataFrame Creation",
                format!("Unable to create DataFrame: {}", e),
                span,
            )),
        }
    }

    pub fn column(&self, column: &str, tag: &Tag) -> Result<Self, ShellError> {
        let s = self
            .as_ref()
            .column(column)
            .map_err(|e| ShellError::labeled_error("Column not found", e.to_string(), tag.span))?;

        let dataframe = DataFrame::new(vec![s.clone()])
            .map_err(|e| ShellError::labeled_error("DataFrame error", e.to_string(), tag.span))?;

        Ok(Self { dataframe })
    }

    pub fn is_series(&self) -> bool {
        self.as_ref().width() == 1
    }

    pub fn as_series(&self, span: &Span) -> Result<Series, ShellError> {
        if !self.is_series() {
            return Err(ShellError::labeled_error_with_secondary(
                "Not a Series",
                "DataFrame cannot be used as Series",
                span,
                "Note that a Series is a DataFrame with one column",
                span,
            ));
        }

        let series = self
            .as_ref()
            .get_columns()
            .get(0)
            .expect("We have already checked that the width is 1");

        Ok(series.clone())
    }

    pub fn get_value(&self, row: usize, span: Span) -> Result<Value, ShellError> {
        let series = self.as_series(&Span::default())?;
        let column = create_column(&series, row, row + 1)?;

        if column.len() == 0 {
            Err(ShellError::labeled_error_with_secondary(
                "Not a valid row",
                format!("No value found for index {}", row),
                span,
                format!("Note that the column size is {}", series.len()),
                span,
            ))
        } else {
            let value = column
                .into_iter()
                .next()
                .expect("already checked there is a value");
            Ok(value)
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
        let upper_row = to_row.min(df.height());

        let mut size: usize = 0;
        let columns = self
            .as_ref()
            .get_columns()
            .iter()
            .map(|col| match create_column(col, from_row, upper_row) {
                Ok(col) => {
                    size = col.len();
                    Ok(col)
                }
                Err(e) => Err(e),
            })
            .collect::<Result<Vec<Column>, ShellError>>()?;

        let mut iterators = columns
            .into_iter()
            .map(|col| (col.name().to_string(), col.into_iter()))
            .collect::<Vec<(String, std::vec::IntoIter<Value>)>>();

        let values = (0..size)
            .into_iter()
            .map(|i| {
                let mut dictionary_row = Dictionary::default();

                for (name, col) in &mut iterators {
                    let dict_val = match col.next() {
                        Some(v) => v,
                        None => {
                            println!("index: {}", i);
                            Value {
                                value: UntaggedValue::Primitive(Primitive::Nothing),
                                tag: Tag::default(),
                            }
                        }
                    };
                    dictionary_row.insert(name.clone(), dict_val);
                }

                Value {
                    value: UntaggedValue::Row(dictionary_row),
                    tag: Tag::unknown(),
                }
            })
            .collect::<Vec<Value>>();

        Ok(values)
    }
}
