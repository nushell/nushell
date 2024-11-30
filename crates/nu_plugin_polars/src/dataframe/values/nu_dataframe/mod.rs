mod between_values;
mod conversion;
mod custom_value;
mod operations;

pub use conversion::{Column, ColumnMap};
pub use operations::Axis;

use indexmap::map::IndexMap;
use nu_protocol::{did_you_mean, PipelineData, Record, ShellError, Span, Value};
use polars::prelude::{
    Column as PolarsColumn, DataFrame, DataType, IntoLazy, PolarsObject, Series,
};
use polars_plan::prelude::{lit, Expr, Null};
use polars_utils::total_ord::{TotalEq, TotalHash};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::Display,
    hash::{Hash, Hasher},
    sync::Arc,
};
use uuid::Uuid;

use crate::{Cacheable, PolarsPlugin};

pub use self::custom_value::NuDataFrameCustomValue;

use super::{
    cant_convert_err, nu_schema::NuSchema, utils::DEFAULT_ROWS, CustomValueSupport, NuLazyFrame,
    PolarsPluginObject, PolarsPluginType,
};

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

impl TotalHash for DataFrameValue {
    fn tot_hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        (*self).hash(state)
    }
}

impl Display for DataFrameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get_type())
    }
}

impl Default for DataFrameValue {
    fn default() -> Self {
        Self(Value::nothing(Span::unknown()))
    }
}

impl PartialEq for DataFrameValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.partial_cmp(&other.0).map_or(false, Ordering::is_eq)
    }
}
impl Eq for DataFrameValue {}

impl Hash for DataFrameValue {
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

impl TotalEq for DataFrameValue {
    fn tot_eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl PolarsObject for DataFrameValue {
    fn type_name() -> &'static str {
        "object"
    }
}

#[derive(Debug, Default, Clone)]
pub struct NuDataFrame {
    pub id: Uuid,
    pub df: Arc<DataFrame>,
    pub from_lazy: bool,
}

impl AsRef<DataFrame> for NuDataFrame {
    fn as_ref(&self) -> &polars::prelude::DataFrame {
        &self.df
    }
}

impl From<DataFrame> for NuDataFrame {
    fn from(df: DataFrame) -> Self {
        Self::new(false, df)
    }
}

impl NuDataFrame {
    pub fn new(from_lazy: bool, df: DataFrame) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            df: Arc::new(df),
            from_lazy,
        }
    }

    pub fn to_polars(&self) -> DataFrame {
        (*self.df).clone()
    }

    pub fn lazy(&self) -> NuLazyFrame {
        NuLazyFrame::new(true, self.to_polars().lazy())
    }

    pub fn try_from_series(series: Series, span: Span) -> Result<Self, ShellError> {
        match DataFrame::new(vec![series.into()]) {
            Ok(dataframe) => Ok(NuDataFrame::new(false, dataframe)),
            Err(e) => Err(ShellError::GenericError {
                error: "Error creating dataframe".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            }),
        }
    }

    pub fn try_from_iter<T>(
        plugin: &PolarsPlugin,
        iter: T,
        maybe_schema: Option<NuSchema>,
    ) -> Result<Self, ShellError>
    where
        T: Iterator<Item = Value>,
    {
        // Dictionary to store the columnar data extracted from
        // the input. During the iteration we check if the values
        // have different type
        let mut column_values: ColumnMap = IndexMap::new();

        for value in iter {
            match value {
                Value::Custom { .. } => {
                    return Self::try_from_value_coerce(plugin, &value, value.span());
                }
                Value::List { vals, .. } => {
                    let record = vals
                        .into_iter()
                        .enumerate()
                        .map(|(i, val)| (format!("{i}"), val))
                        .collect();

                    conversion::insert_record(&mut column_values, record, &maybe_schema)?
                }
                Value::Record { val: record, .. } => conversion::insert_record(
                    &mut column_values,
                    record.into_owned(),
                    &maybe_schema,
                )?,
                _ => {
                    let key = "0".to_string();
                    conversion::insert_value(value, key.into(), &mut column_values, &maybe_schema)?
                }
            }
        }

        let df = conversion::from_parsed_columns(column_values)?;
        add_missing_columns(df, &maybe_schema, Span::unknown())
    }

    pub fn try_from_series_vec(columns: Vec<Series>, span: Span) -> Result<Self, ShellError> {
        let columns_converted: Vec<PolarsColumn> = columns.into_iter().map(Into::into).collect();

        let dataframe =
            DataFrame::new(columns_converted).map_err(|e| ShellError::GenericError {
                error: "Error creating dataframe".into(),
                msg: format!("Unable to create DataFrame: {e}"),
                span: Some(span),
                help: None,
                inner: vec![],
            })?;

        Ok(Self::new(false, dataframe))
    }

    pub fn try_from_columns(
        columns: Vec<Column>,
        maybe_schema: Option<NuSchema>,
    ) -> Result<Self, ShellError> {
        let mut column_values: ColumnMap = IndexMap::new();

        for column in columns {
            let name = column.name().clone();
            for value in column {
                conversion::insert_value(value, name.clone(), &mut column_values, &maybe_schema)?;
            }
        }

        let df = conversion::from_parsed_columns(column_values)?;
        add_missing_columns(df, &maybe_schema, Span::unknown())
    }

    pub fn fill_list_nan(list: Vec<Value>, list_span: Span, fill: Value) -> Value {
        let newlist = list
            .into_iter()
            .map(|value| {
                let span = value.span();
                match value {
                    Value::Float { val, .. } => {
                        if val.is_nan() {
                            fill.clone()
                        } else {
                            value
                        }
                    }
                    Value::List { vals, .. } => Self::fill_list_nan(vals, span, fill.clone()),
                    _ => value,
                }
            })
            .collect::<Vec<Value>>();
        Value::list(newlist, list_span)
    }

    pub fn columns(&self, span: Span) -> Result<Vec<Column>, ShellError> {
        let height = self.df.height();
        self.df
            .get_columns()
            .iter()
            .map(|col| conversion::create_column(col, 0, height, span))
            .collect::<Result<Vec<Column>, ShellError>>()
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
            ShellError::DidYouMean {
                suggestion: option,
                span,
            }
        })?;

        let df = DataFrame::new(vec![s.clone()]).map_err(|e| ShellError::GenericError {
            error: "Error creating dataframe".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;

        Ok(Self::new(false, df))
    }

    pub fn is_series(&self) -> bool {
        self.df.width() == 1
    }

    pub fn as_series(&self, span: Span) -> Result<Series, ShellError> {
        if !self.is_series() {
            return Err(ShellError::GenericError {
                error: "Error using as series".into(),
                msg: "dataframe has more than one column".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            });
        }

        let series = self
            .df
            .get_columns()
            .first()
            .expect("We have already checked that the width is 1")
            .as_materialized_series();

        Ok(series.clone())
    }

    pub fn get_value(&self, row: usize, span: Span) -> Result<Value, ShellError> {
        let series = self.as_series(span)?;
        let column = conversion::create_column_from_series(&series, row, row + 1, span)?;

        if column.len() == 0 {
            Err(ShellError::AccessEmptyContent { span })
        } else {
            let value = column
                .into_iter()
                .next()
                .expect("already checked there is a value");
            Ok(value)
        }
    }

    pub fn has_index(&self) -> bool {
        self.columns(Span::unknown())
            .unwrap_or_default() // just assume there isn't an index
            .iter()
            .any(|col| col.name() == "index")
    }

    // Print is made out a head and if the dataframe is too large, then a tail
    pub fn print(&self, include_index: bool, span: Span) -> Result<Vec<Value>, ShellError> {
        let df = &self.df;
        let size: usize = 20;

        if df.height() > size {
            let sample_size = size / 2;
            let mut values = self.head(Some(sample_size), include_index, span)?;
            conversion::add_separator(&mut values, df, self.has_index(), span);
            let remaining = df.height() - sample_size;
            let tail_size = remaining.min(sample_size);
            let mut tail_values = self.tail(Some(tail_size), include_index, span)?;
            values.append(&mut tail_values);

            Ok(values)
        } else {
            Ok(self.head(Some(size), include_index, span)?)
        }
    }

    pub fn height(&self) -> usize {
        self.df.height()
    }

    pub fn head(
        &self,
        rows: Option<usize>,
        include_index: bool,
        span: Span,
    ) -> Result<Vec<Value>, ShellError> {
        let to_row = rows.unwrap_or(5);
        let values = self.to_rows(0, to_row, include_index, span)?;
        Ok(values)
    }

    pub fn tail(
        &self,
        rows: Option<usize>,
        include_index: bool,
        span: Span,
    ) -> Result<Vec<Value>, ShellError> {
        let df = &self.df;
        let to_row = df.height();
        let size = rows.unwrap_or(DEFAULT_ROWS);
        let from_row = to_row.saturating_sub(size);

        let values = self.to_rows(from_row, to_row, include_index, span)?;
        Ok(values)
    }

    /// Converts the dataframe to a nushell list of values
    pub fn to_rows(
        &self,
        from_row: usize,
        to_row: usize,
        include_index: bool,
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

        let has_index = self.has_index();
        let values = (0..size)
            .map(|i| {
                let mut record = Record::new();

                if !has_index && include_index {
                    record.push("index", Value::int((i + from_row) as i64, span));
                }

                for (name, col) in &mut iterators {
                    record.push(name.clone(), col.next().unwrap_or(Value::nothing(span)));
                }

                Value::record(record, span)
            })
            .collect::<Vec<Value>>();

        Ok(values)
    }

    // Dataframes are considered equal if they have the same shape, column name and values
    pub fn is_equal(&self, other: &Self) -> Option<Ordering> {
        let polars_self = self.to_polars();
        let polars_other = other.to_polars();

        if polars_self == polars_other {
            Some(Ordering::Equal)
        } else {
            None
        }
    }

    pub fn schema(&self) -> NuSchema {
        NuSchema::new(self.df.schema())
    }

    /// This differs from try_from_value as it will attempt to coerce the type into a NuDataFrame.
    /// So, if the pipeline type is a NuLazyFrame it will be collected and returned as NuDataFrame.
    pub fn try_from_value_coerce(
        plugin: &PolarsPlugin,
        value: &Value,
        span: Span,
    ) -> Result<Self, ShellError> {
        match PolarsPluginObject::try_from_value(plugin, value)? {
            PolarsPluginObject::NuDataFrame(df) => Ok(df),
            PolarsPluginObject::NuLazyFrame(lazy) => lazy.collect(span),
            _ => Err(cant_convert_err(
                value,
                &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
            )),
        }
    }

    /// This differs from try_from_pipeline as it will attempt to coerce the type into a NuDataFrame.
    /// So, if the pipeline type is a NuLazyFrame it will be collected and returned as NuDataFrame.
    pub fn try_from_pipeline_coerce(
        plugin: &PolarsPlugin,
        input: PipelineData,
        span: Span,
    ) -> Result<Self, ShellError> {
        let value = input.into_value(span)?;
        Self::try_from_value_coerce(plugin, &value, span)
    }
}

fn add_missing_columns(
    df: NuDataFrame,
    maybe_schema: &Option<NuSchema>,
    span: Span,
) -> Result<NuDataFrame, ShellError> {
    // If there are fields that are in the schema, but not in the dataframe
    // add them to the dataframe.
    if let Some(schema) = maybe_schema {
        let fields = df.df.fields();
        let df_field_names: HashSet<&str> = fields.iter().map(|f| f.name().as_str()).collect();

        let missing: Vec<(&str, &DataType)> = schema
            .schema
            .iter()
            .filter_map(|(name, dtype)| {
                let name = name.as_str();
                if !df_field_names.contains(name) {
                    Some((name, dtype))
                } else {
                    None
                }
            })
            .collect();

        let missing_exprs: Vec<Expr> = missing
            .iter()
            .map(|(name, dtype)| lit(Null {}).cast((*dtype).to_owned()).alias(*name))
            .collect();

        let df = if !missing.is_empty() {
            let lazy: NuLazyFrame = df.lazy().to_polars().with_columns(missing_exprs).into();
            lazy.collect(span)?
        } else {
            df
        };
        Ok(df)
    } else {
        Ok(df)
    }
}

impl Cacheable for NuDataFrame {
    fn cache_id(&self) -> &Uuid {
        &self.id
    }

    fn to_cache_value(&self) -> Result<PolarsPluginObject, ShellError> {
        Ok(PolarsPluginObject::NuDataFrame(self.clone()))
    }

    fn from_cache_value(cv: PolarsPluginObject) -> Result<Self, ShellError> {
        match cv {
            PolarsPluginObject::NuDataFrame(df) => Ok(df),
            _ => Err(ShellError::GenericError {
                error: "Cache value is not a dataframe".into(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }),
        }
    }
}

impl CustomValueSupport for NuDataFrame {
    type CV = NuDataFrameCustomValue;

    fn custom_value(self) -> Self::CV {
        NuDataFrameCustomValue {
            id: self.id,
            dataframe: Some(self),
        }
    }

    fn base_value(self, span: Span) -> Result<Value, ShellError> {
        let vals = self.print(true, span)?;
        Ok(Value::list(vals, span))
    }

    fn get_type_static() -> PolarsPluginType {
        PolarsPluginType::NuDataFrame
    }
}
