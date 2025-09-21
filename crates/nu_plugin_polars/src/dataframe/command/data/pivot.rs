use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use chrono::DateTime;
use polars::prelude::Expr;
use polars_lazy::frame::pivot::{pivot, pivot_stable};

use crate::{
    PolarsPlugin,
    dataframe::values::utils::convert_columns_string,
    values::{Column, CustomValueSupport, NuExpression, PolarsPluginObject},
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
        "Pivot a DataFrame from long to wide format."
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
                SyntaxShape::Any,
                "Aggregation to apply when pivoting. The following are supported: first, sum, min, max, mean, median, count, last, or a custom expression",
                Some('a'),
            )
            .named(
                "separator",
                SyntaxShape::String,
                "Delimiter in generated column names in case of multiple `values` columns (default '_')",
                Some('p'),
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
            .switch(
                "stable",
                "Perform a stable pivot.",
                None,
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Perform a pivot in order to show individuals test score by subject",
                example: "[[name subject date test_1 test_2]; [Cady maths 2025-04-01 98 100] [Cady physics 2025-04-01 99 100] [Karen maths 2025-04-02 61 60] [Karen physics 2025-04-02 58 60]] | polars into-df |  polars pivot --on [subject] --index [name date] --values [test_1]",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "name".to_string(),
                                vec![
                                    Value::string("Cady", Span::test_data()),
                                    Value::string("Karen", Span::test_data()),
                                ],
                            ),
                            Column::new(
                                "date".to_string(),
                                vec![
                                    Value::date(
                                        DateTime::parse_from_str(
                                            "2025-04-01 00:00:00 +0000",
                                            "%Y-%m-%d %H:%M:%S %z",
                                        )
                                        .expect("date calculation should not fail in test"),
                                        Span::test_data(),
                                    ),
                                    Value::date(
                                        DateTime::parse_from_str(
                                            "2025-04-02 00:00:00 +0000",
                                            "%Y-%m-%d %H:%M:%S %z",
                                        )
                                        .expect("date calculation should not fail in test"),
                                        Span::test_data(),
                                    ),
                                ],
                            ),
                            Column::new(
                                "maths".to_string(),
                                vec![
                                    Value::int(98, Span::test_data()),
                                    Value::int(61, Span::test_data()),
                                ],
                            ),
                            Column::new(
                                "physics".to_string(),
                                vec![
                                    Value::int(99, Span::test_data()),
                                    Value::int(58, Span::test_data()),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
            Example {
                description: "Perform a pivot with multiple `values` columns with a separator",
                example: "[[name subject date test_1 test_2 grade_1 grade_2]; [Cady maths 2025-04-01 98 100 A A] [Cady physics 2025-04-01 99 100 A A] [Karen maths 2025-04-02 61 60 D D] [Karen physics 2025-04-02 58 60 D D]] | polars into-df |  polars pivot --on [subject] --index [name] --values [test_1 grade_1] --separator /",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "name".to_string(),
                                vec![
                                    Value::string("Cady", Span::test_data()),
                                    Value::string("Karen", Span::test_data()),
                                ],
                            ),
                            Column::new(
                                "test_1/maths".to_string(),
                                vec![
                                    Value::int(98, Span::test_data()),
                                    Value::int(61, Span::test_data()),
                                ],
                            ),
                            Column::new(
                                "test_1/physics".to_string(),
                                vec![
                                    Value::int(99, Span::test_data()),
                                    Value::int(58, Span::test_data()),
                                ],
                            ),
                            Column::new(
                                "grade_1/maths".to_string(),
                                vec![
                                    Value::string("A", Span::test_data()),
                                    Value::string("D", Span::test_data()),
                                ],
                            ),
                            Column::new(
                                "grade_1/physics".to_string(),
                                vec![
                                    Value::string("A", Span::test_data()),
                                    Value::string("D", Span::test_data()),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
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
        .map(|pd| pd.set_metadata(metadata))
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

    let (on_col_string, ..) = convert_columns_string(on_col, call.head)?;
    let (index_col_string, ..) = convert_columns_string(index_col, call.head)?;
    let (val_col_string, ..) = convert_columns_string(val_col, call.head)?;

    let aggregate: Option<Expr> = call
        .get_flag::<Value>("aggregate")?
        .map(|val| pivot_agg_for_value(plugin, val))
        .transpose()?;

    let separator: Option<String> = call.get_flag::<String>("separator")?;

    let sort = call.has_flag("sort")?;
    let stable = call.has_flag("stable")?;
    let polars_df = df.to_polars();

    let pivoted = if stable {
        pivot_stable(
            &polars_df,
            &on_col_string,
            Some(&index_col_string),
            Some(&val_col_string),
            sort,
            aggregate,
            separator.as_deref(),
        )
    } else {
        pivot(
            &polars_df,
            &on_col_string,
            Some(&index_col_string),
            Some(&val_col_string),
            sort,
            aggregate,
            separator.as_deref(),
        )
    }
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

#[allow(dead_code)]
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

fn pivot_agg_for_value(plugin: &PolarsPlugin, agg: Value) -> Result<Expr, ShellError> {
    match agg {
        Value::String { val, .. } => match val.as_str() {
            "first" => Ok(polars::prelude::first().as_expr()),
            "sum" => Ok(polars::prelude::sum("*")),
            "min" => Ok(polars::prelude::min("*")),
            "max" => Ok(polars::prelude::max("*")),
            "mean" => Ok(polars::prelude::mean("*")),
            "median" => Ok(polars::prelude::median("*")),
            "count" => Ok(polars::prelude::len()),
            "len" => Ok(polars::prelude::len()),
            "last" => Ok(polars::prelude::last().as_expr()),
            s => Err(ShellError::GenericError {
                error: format!("{s} is not a valid aggregation"),
                msg: "".into(),
                span: None,
                help: Some(
                    "Use one of the following: first, sum, min, max, mean, median, count, last"
                        .into(),
                ),
                inner: vec![],
            }),
        },
        Value::Custom { .. } => {
            let expr = NuExpression::try_from_value(plugin, &agg)?;
            Ok(expr.into_polars())
        }
        _ => Err(ShellError::GenericError {
            error: "Aggregation must be a string or expression".into(),
            msg: "".into(),
            span: Some(agg.span()),
            help: None,
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
