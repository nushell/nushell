use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use polars_ops::pivot::{pivot, PivotAgg};

use crate::{
    dataframe::values::utils::convert_columns_string,
    values::{Column, CustomValueSupport, PolarsPluginObject},
    PolarsPlugin,
};

use crate::values::NuDataFrame;

#[derive(Clone)]
pub struct PivotDF;

impl PluginCommand for PivotDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars pivot"
    }

    fn description(&self) -> &str {
        "Pivot a DataFrame from wide to long format."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required_named(
                "on",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names for pivoting",
                Some('o'),
            )
            .required_named(
                "index",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names for indexes",
                Some('i'),
            )
            .required_named(
                "values",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "column names used as value columns",
                Some('v'),
            )
            .named(
                "aggregate",
                SyntaxShape::String,
                "Aggregation to apply when pivoting. The following are supported: first, sum, min, max, mean, median, count, last",
                Some('a'),
            )
            .switch(
                "sort",
                "Sort columns",
                Some('s'),
            )
            .switch(
                "streamable",
                "Whether or not to use the polars streaming engine. Only valid for lazy dataframes",
                Some('t'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "[[name subject test_1 test_2]; [Cady maths 98 100] [Cady physics 99 100] [Karen maths 61 60] [Karen physics 58 60]] | polars into-df |  polars pivot --on [subject] --index [name] --values [test_1]",
                description: "Perform a pivot in order to show individuals test score by subject",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "name".to_string(),
                                vec![Value::string("Cady", Span::test_data()), Value::string("Karen", Span::test_data())],
                            ),
                            Column::new(
                                "maths".to_string(),
                                vec![Value::int(98, Span::test_data()), Value::int(61, Span::test_data())],
                            ),
                            Column::new(
                                "physics".to_string(),
                                vec![Value::int(99, Span::test_data()), Value::int(58, Span::test_data())],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown())
                )
            }
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        match PolarsPluginObject::try_from_pipeline(plugin, input, call.head)? {
            PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command_eager(plugin, engine, call, lazy.collect(call.head)?)
            }
            _ => Err(ShellError::GenericError {
                error: "Must be a dataframe or lazy dataframe".into(),
                msg: "".into(),
                span: Some(call.head),
                help: None,
                inner: vec![],
            }),
        }
        .map_err(LabeledError::from)
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let on_col: Vec<Value> = call.get_flag("on")?.expect("required value");
    let index_col: Vec<Value> = call.get_flag("index")?.expect("required value");
    let val_col: Vec<Value> = call.get_flag("values")?.expect("required value");

    let (on_col_string, id_col_span) = convert_columns_string(on_col, call.head)?;
    let (index_col_string, index_col_span) = convert_columns_string(index_col, call.head)?;
    let (val_col_string, val_col_span) = convert_columns_string(val_col, call.head)?;

    check_column_datatypes(df.as_ref(), &on_col_string, id_col_span)?;
    check_column_datatypes(df.as_ref(), &index_col_string, index_col_span)?;
    check_column_datatypes(df.as_ref(), &val_col_string, val_col_span)?;

    let aggregate: Option<PivotAgg> = call
        .get_flag::<String>("aggregate")?
        .map(pivot_agg_for_str)
        .transpose()?;

    let sort = call.has_flag("sort")?;

    let polars_df = df.to_polars();
    // todo add other args
    let pivoted = pivot(
        &polars_df,
        &on_col_string,
        Some(&index_col_string),
        Some(&val_col_string),
        sort,
        aggregate,
        None,
    )
    .map_err(|e| ShellError::GenericError {
        error: format!("Pivot error: {e}"),
        msg: "".into(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = NuDataFrame::new(false, pivoted);
    res.to_pipeline_data(plugin, engine, call.head)
}

fn check_column_datatypes<T: AsRef<str>>(
    df: &polars::prelude::DataFrame,
    cols: &[T],
    col_span: Span,
) -> Result<(), ShellError> {
    if cols.is_empty() {
        return Err(ShellError::GenericError {
            error: "Merge error".into(),
            msg: "empty column list".into(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        });
    }

    // Checking if they are same type
    if cols.len() > 1 {
        for w in cols.windows(2) {
            let l_series = df
                .column(w[0].as_ref())
                .map_err(|e| ShellError::GenericError {
                    error: "Error selecting columns".into(),
                    msg: e.to_string(),
                    span: Some(col_span),
                    help: None,
                    inner: vec![],
                })?;

            let r_series = df
                .column(w[1].as_ref())
                .map_err(|e| ShellError::GenericError {
                    error: "Error selecting columns".into(),
                    msg: e.to_string(),
                    span: Some(col_span),
                    help: None,
                    inner: vec![],
                })?;

            if l_series.dtype() != r_series.dtype() {
                return Err(ShellError::GenericError {
                    error: "Merge error".into(),
                    msg: "found different column types in list".into(),
                    span: Some(col_span),
                    help: Some(format!(
                        "datatypes {} and {} are incompatible",
                        l_series.dtype(),
                        r_series.dtype()
                    )),
                    inner: vec![],
                });
            }
        }
    }

    Ok(())
}

fn pivot_agg_for_str(agg: impl AsRef<str>) -> Result<PivotAgg, ShellError> {
    match agg.as_ref() {
        "first" => Ok(PivotAgg::First),
        "sum" => Ok(PivotAgg::Sum),
        "min" => Ok(PivotAgg::Min),
        "max" => Ok(PivotAgg::Max),
        "mean" => Ok(PivotAgg::Mean),
        "median" => Ok(PivotAgg::Median),
        "count" => Ok(PivotAgg::Count),
        "last" => Ok(PivotAgg::Last),
        s => Err(ShellError::GenericError {
            error: format!("{s} is not a valid aggregation"),
            msg: "".into(),
            span: None,
            help: Some(
                "Use one of the following: first, sum, min, max, mean, median, count, last".into(),
            ),
            inner: vec![],
        }),
    }
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&PivotDF)
    }
}
