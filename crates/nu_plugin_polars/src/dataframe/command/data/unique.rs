use crate::{
    PolarsPlugin,
    dataframe::{utils::extract_strings, values::NuLazyFrame},
    values::{
        CustomValueSupport, NuExpression, PolarsPluginObject, PolarsPluginType, cant_convert_err,
    },
};

use crate::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{IntoSeries, UniqueKeepStrategy, cols};

#[derive(Clone)]
pub struct Unique;

impl PluginCommand for Unique {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars unique"
    }

    fn description(&self) -> &str {
        "Returns unique values from a dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "subset",
                SyntaxShape::Any,
                "Subset of column(s) to use to maintain rows (lazy df)",
                Some('s'),
            )
            .switch(
                "last",
                "Keeps last unique value. Default keeps first value (lazy df)",
                Some('l'),
            )
            .switch(
                "maintain-order",
                "Keep the same order as the original DataFrame (lazy df)",
                Some('k'),
            )
            .input_output_types(vec![
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
            ])
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Returns unique values from a series",
                example: "[2 2 2 2 2] | polars into-df | polars unique",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new("0".to_string(), vec![Value::test_int(2)])],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns unique values in a subset of lazyframe columns",
                example: "[[a b c]; [1 2 1] [2 2 2] [3 2 1]] | polars into-lazy | polars unique --subset [b c] | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(2)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(2)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(1), Value::test_int(2)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns unique values in a subset of lazyframe columns",
                example: r#"[[a b c]; [1 2 1] [2 2 2] [3 2 1]]
    | polars into-lazy
    | polars unique --subset [b c] --last
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(2), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(2)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(2), Value::test_int(1)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns unique values in a subset of lazyframe columns",
                example: r#"[[a]; [2] [1] [2]]
    | polars into-lazy
    | polars select (polars col a | polars unique)
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "a".to_string(),
                            vec![Value::test_int(1), Value::test_int(2)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns unique values in a subset of lazyframe columns",
                example: r#"[[a]; [2] [1] [2]]
    | polars into-lazy
    | polars select (polars col a | polars unique --maintain-order)
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "a".to_string(),
                            vec![Value::test_int(2), Value::test_int(1)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
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
        let value = input.into_value(call.head)?;

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            PolarsPluginObject::NuExpression(expr) => {
                let maintain = call.has_flag("maintain-order")?;
                let res: NuExpression = if maintain {
                    expr.into_polars().unique_stable().into()
                } else {
                    expr.into_polars().unique().into()
                };
                res.to_pipeline_data(plugin, engine, call.head)
            }
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyGroupBy,
                    PolarsPluginType::NuExpression,
                ],
            )),
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
    let series = df.as_series(call.head)?;

    let res = series.unique().map_err(|e| ShellError::GenericError {
        error: "Error calculating unique values".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: Some("The str-slice command can only be used with string columns".into()),
        inner: vec![],
    })?;

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let last = call.has_flag("last")?;
    let maintain = call.has_flag("maintain-order")?;
    // todo: allow selectors to be passed in
    let subset: Option<Value> = call.get_flag("subset")?;
    let subset = match subset {
        Some(value) => Some(cols(extract_strings(value)?)),
        None => None,
    };

    let strategy = if last {
        UniqueKeepStrategy::Last
    } else {
        UniqueKeepStrategy::First
    };

    let lazy = lazy.to_polars();
    let lazy: NuLazyFrame = if maintain {
        lazy.unique(subset, strategy).into()
    } else {
        lazy.unique_stable(subset, strategy).into()
    };
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Unique)
    }
}
