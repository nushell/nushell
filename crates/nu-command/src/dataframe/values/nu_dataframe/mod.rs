mod between_values;
mod conversion;
mod custom_value;
mod operations;

pub use conversion::{Column, ColumnMap};
pub use operations::Axis;

use indexmap::map::IndexMap;
use nu_protocol::{did_you_mean, PipelineData, ShellError, Span, Value};
use polars::prelude::{DataFrame, DataType, IntoLazy, LazyFrame, PolarsObject, Series};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Display, hash::Hasher};

use super::{utils::DEFAULT_ROWS, NuLazyFrame};

// DataFrameValue is an encapsulation of Nushell Value that can be used
// to define the PolarsObject Trait. The polars object trait allows to
// create dataframes with mixed datatypes
#[derive(Clone, Debug)]
pub struct DataFrameValue(Value);

impl DataFrameValue {
    fn new(value: Value) -> Self {
        Self(value)
    }

    fn get_value(&self) -> Value {
        self.0.clone()
    }
}

impl Display for DataFrameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get_type())
    }
}

impl Default for DataFrameValue {
    fn default() -> Self {
        Self(Value::Nothing {
            span: Span { start: 0, end: 0 },
        })
    }
}

impl PartialEq for DataFrameValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.partial_cmp(&other.0).map_or(false, Ordering::is_eq)
    }
}
impl Eq for DataFrameValue {}

impl std::hash::Hash for DataFrameValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.0 {
            Value::Nothing { .. } => 0.hash(state),
            Value::Int { val, .. } => val.hash(state),
            Value::String { val, .. } => val.hash(state),
            // TODO. Define hash for the rest of types
            _ => {}
        }
    }
}

impl PolarsObject for DataFrameValue {
    fn type_name() -> &'static str {
        "object"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NuDataFrame {
    pub df: DataFrame,
    pub from_lazy: bool,
}

impl AsRef<DataFrame> for NuDataFrame {
    fn as_ref(&self) -> &polars::prelude::DataFrame {
        &self.df
    }
}

impl AsMut<DataFrame> for NuDataFrame {
    fn as_mut(&mut self) -> &mut polars::prelude::DataFrame {
        &mut self.df
    }
}

impl From<DataFrame> for NuDataFrame {
    fn from(df: DataFrame) -> Self {
        Self {
            df,
            from_lazy: false,
        }
    }
}

impl NuDataFrame {
    pub fn new(from_lazy: bool, df: DataFrame) -> Self {
        Self { df, from_lazy }
    }

    pub fn lazy(&self) -> LazyFrame {
        self.df.clone().lazy()
    }

    fn default_value(span: Span) -> Value {
        let dataframe = DataFrame::default();
        NuDataFrame::dataframe_into_value(dataframe, span)
    }

    pub fn dataframe_into_value(dataframe: DataFrame, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(Self::new(false, dataframe)),
            span,
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        if self.from_lazy {
            let lazy = NuLazyFrame::from_dataframe(self);
            Value::CustomValue {
                val: Box::new(lazy),
                span,
            }
        } else {
            Value::CustomValue {
                val: Box::new(self),
                span,
            }
        }
    }

    pub fn series_to_value(series: Series, span: Span) -> Result<Value, ShellError> {
        match DataFrame::new(vec![series]) {
            Ok(dataframe) => Ok(NuDataFrame::dataframe_into_value(dataframe, span)),
            Err(e) => Err(ShellError::GenericError(
                "Error creating dataframe".into(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            )),
        }
    }

    pub fn try_from_iter<T>(iter: T) -> Result<Self, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        // Dictionary to store the columnar data extracted from
        // the input. During the iteration we check if the values
        // have different type
        let mut column_values: ColumnMap = IndexMap::new();

        for value in iter {
            match value {
                Value::CustomValue { .. } => return Self::try_from_value(value),
                Value::List { vals, .. } => {
                    let cols = (0..vals.len())
                        .map(|i| format!("{}", i))
                        .collect::<Vec<String>>();

                    conversion::insert_record(&mut column_values, &cols, &vals)?
                }
                Value::Record { cols, vals, .. } => {
                    conversion::insert_record(&mut column_values, &cols, &vals)?
                }
                _ => {
                    let key = "0".to_string();
                    conversion::insert_value(value, key, &mut column_values)?
                }
            }
        }

        conversion::from_parsed_columns(column_values)
    }

    pub fn try_from_series(columns: Vec<Series>, span: Span) -> Result<Self, ShellError> {
        let dataframe = DataFrame::new(columns).map_err(|e| {
            ShellError::GenericError(
                "Error creating dataframe".into(),
                format!("Unable to create DataFrame: {}", e),
                Some(span),
                None,
                Vec::new(),
            )
        })?;

        Ok(Self::new(false, dataframe))
    }

    pub fn try_from_columns(columns: Vec<Column>) -> Result<Self, ShellError> {
        let mut column_values: ColumnMap = IndexMap::new();

        for column in columns {
            let name = column.name().to_string();
            for value in column {
                conversion::insert_value(value, name.clone(), &mut column_values)?;
            }
        }

        conversion::from_parsed_columns(column_values)
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        if Self::can_downcast(&value) {
            Ok(Self::get_df(value)?)
        } else if NuLazyFrame::can_downcast(&value) {
            let span = value.span()?;
            let lazy = NuLazyFrame::try_from_value(value)?;
            let df = lazy.collect(span)?;
            Ok(df)
        } else {
            Err(ShellError::CantConvert(
                "lazy or eager dataframe".into(),
                value.get_type().to_string(),
                value.span()?,
                None,
            ))
        }
    }

    pub fn get_df(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(df) => Ok(NuDataFrame {
                    df: df.df.clone(),
                    from_lazy: false,
                }),
                None => Err(ShellError::CantConvert(
                    "dataframe".into(),
                    "non-dataframe".into(),
                    span,
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "dataframe".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn can_downcast(value: &Value) -> bool {
        if let Value::CustomValue { val, .. } = value {
            val.as_any().downcast_ref::<Self>().is_some()
        } else {
            false
        }
    }

    pub fn column(&self, column: &str, span: Span) -> Result<Self, ShellError> {
        let s = self.df.column(column).map_err(|_| {
            let possibilities = self
                .df
                .get_column_names()
                .iter()
                .map(|name| name.to_string())
                .collect::<Vec<String>>();

            let option = did_you_mean(&possibilities, column).unwrap_or_else(|| column.to_string());
            ShellError::DidYouMean(option, span)
        })?;

        let df = DataFrame::new(vec![s.clone()]).map_err(|e| {
            ShellError::GenericError(
                "Error creating dataframe".into(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            )
        })?;

        Ok(Self {
            df,
            from_lazy: false,
        })
    }

    pub fn is_series(&self) -> bool {
        self.df.width() == 1
    }

    pub fn as_series(&self, span: Span) -> Result<Series, ShellError> {
        if !self.is_series() {
            return Err(ShellError::GenericError(
                "Error using as series".into(),
                "dataframe has more than one column".into(),
                Some(span),
                None,
                Vec::new(),
            ));
        }

        let series = self
            .df
            .get_columns()
            .get(0)
            .expect("We have already checked that the width is 1");

        Ok(series.clone())
    }

    pub fn get_value(&self, row: usize, span: Span) -> Result<Value, ShellError> {
        let series = self.as_series(span)?;
        let column = conversion::create_column(&series, row, row + 1, span)?;

        if column.len() == 0 {
            Err(ShellError::AccessEmptyContent(span))
        } else {
            let value = column
                .into_iter()
                .next()
                .expect("already checked there is a value");
            Ok(value)
        }
    }

    // Print is made out a head and if the dataframe is too large, then a tail
    pub fn print(&self, span: Span) -> Result<Vec<Value>, ShellError> {
        let df = &self.df;
        let size: usize = 20;

        if df.height() > size {
            let sample_size = size / 2;
            let mut values = self.head(Some(sample_size), span)?;
            conversion::add_separator(&mut values, df, span);
            let remaining = df.height() - sample_size;
            let tail_size = remaining.min(sample_size);
            let mut tail_values = self.tail(Some(tail_size), span)?;
            values.append(&mut tail_values);

            Ok(values)
        } else {
            Ok(self.head(Some(size), span)?)
        }
    }

    pub fn height(&self) -> usize {
        self.df.height()
    }

    pub fn head(&self, rows: Option<usize>, span: Span) -> Result<Vec<Value>, ShellError> {
        let to_row = rows.unwrap_or(5);
        let values = self.to_rows(0, to_row, span)?;

        Ok(values)
    }

    pub fn tail(&self, rows: Option<usize>, span: Span) -> Result<Vec<Value>, ShellError> {
        let df = &self.df;
        let to_row = df.height();
        let size = rows.unwrap_or(DEFAULT_ROWS);
        let from_row = to_row.saturating_sub(size);

        let values = self.to_rows(from_row, to_row, span)?;

        Ok(values)
    }

    pub fn to_rows(
        &self,
        from_row: usize,
        to_row: usize,
        span: Span,
    ) -> Result<Vec<Value>, ShellError> {
        let df = &self.df;
        let upper_row = to_row.min(df.height());

        let mut size: usize = 0;
        let columns = self
            .df
            .get_columns()
            .iter()
            .map(
                |col| match conversion::create_column(col, from_row, upper_row, span) {
                    Ok(col) => {
                        size = col.len();
                        Ok(col)
                    }
                    Err(e) => Err(e),
                },
            )
            .collect::<Result<Vec<Column>, ShellError>>()?;

        let mut iterators = columns
            .into_iter()
            .map(|col| (col.name().to_string(), col.into_iter()))
            .collect::<Vec<(String, std::vec::IntoIter<Value>)>>();

        let values = (0..size)
            .into_iter()
            .map(|i| {
                let mut cols = vec![];
                let mut vals = vec![];

                cols.push("index".into());
                vals.push(Value::Int {
                    val: (i + from_row) as i64,
                    span,
                });

                for (name, col) in &mut iterators {
                    cols.push(name.clone());

                    match col.next() {
                        Some(v) => vals.push(v),
                        None => vals.push(Value::Nothing { span }),
                    };
                }

                Value::Record { cols, vals, span }
            })
            .collect::<Vec<Value>>();

        Ok(values)
    }

    // Dataframes are considered equal if they have the same shape, column name and values
    pub fn is_equal(&self, other: &Self) -> Option<Ordering> {
        if self.as_ref().width() == 0 {
            // checking for empty dataframe
            return None;
        }

        if self.as_ref().get_column_names() != other.as_ref().get_column_names() {
            // checking both dataframes share the same names
            return None;
        }

        if self.as_ref().height() != other.as_ref().height() {
            // checking both dataframes have the same row size
            return None;
        }

        // sorting dataframe by the first column
        let column_names = self.as_ref().get_column_names();
        let first_col = column_names
            .first()
            .expect("already checked that dataframe is different than 0");

        // if unable to sort, then unable to compare
        let lhs = match self.as_ref().sort(vec![*first_col], false) {
            Ok(df) => df,
            Err(_) => return None,
        };

        let rhs = match other.as_ref().sort(vec![*first_col], false) {
            Ok(df) => df,
            Err(_) => return None,
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
                DataType::UInt32 | DataType::Int32 => match self_series.cast(&DataType::Int64) {
                    Ok(series) => series,
                    Err(_) => return None,
                },
                _ => self_series.clone(),
            };

            if !self_series.series_equal(other_series) {
                return None;
            }
        }

        Some(Ordering::Equal)
    }
}
