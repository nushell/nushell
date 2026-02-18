use crate::values::{Column, NuDataFrame, PolarsPluginType};
use crate::{
    PolarsPlugin,
    dataframe::values::{NuExpression, NuLazyFrame},
    values::{CustomValueSupport, PolarsPluginObject},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Spanned,
    SyntaxShape, Value,
};

#[derive(Clone)]
pub struct WithColumn;

impl PluginCommand for WithColumn {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars with-column"
    }

    fn description(&self) -> &str {
        "Adds a series to the dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named("name", SyntaxShape::String, "New column name. For lazy dataframes and expressions syntax, use a `polars as` expression to name a column.", Some('n'))
            .rest(
                "series or expressions",
                SyntaxShape::Any,
                "series to be added or expressions used to define the new columns",
            )
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Adds a series to the dataframe",
                example: r#"[[a b]; [1 2] [3 4]]
    | polars into-df
    | polars with-column ([5 6] | polars into-df) --name c"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(5), Value::test_int(6)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Adds a series to the dataframe",
                example: r#"[[a b]; [1 2] [3 4]]
    | polars into-lazy
    | polars with-column [
        ((polars col a) * 2 | polars as "c")
        ((polars col a) * 3 | polars as "d")
      ]
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(2), Value::test_int(6)],
                            ),
                            Column::new(
                                "d".to_string(),
                                vec![Value::test_int(3), Value::test_int(9)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Add series to a lazyframe using a record",
                example: r#"[[a b]; [1 2] [3 4]]
    | polars into-lazy
    | polars with-column {
        c: ((polars col a) * 2)
        d: ((polars col a) * 3)
      }
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(2), Value::test_int(6)],
                            ),
                            Column::new(
                                "d".to_string(),
                                vec![Value::test_int(3), Value::test_int(9)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Add series to a dataframe using a record",
                example: r#"[[a b]; [1 2] [3 4]]
    | polars into-df
    | polars with-column {
        c: ((polars col a) * 2)
        d: ((polars col a) * 3)
      }
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(2), Value::test_int(6)],
                            ),
                            Column::new(
                                "d".to_string(),
                                vec![Value::test_int(3), Value::test_int(9)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Add columns using a selector to multiply all columns by 2",
                example: r#"[[a b]; [1 2] [3 4]]
    | polars into-df
    | polars with-column ((polars selector all) * 2)
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(2), Value::test_int(6)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(4), Value::test_int(8)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Add a new column using a selector on the first column",
                example: r#"[[a b c]; [1 2 3] [4 5 6]]
    | polars into-df
    | polars with-column ((polars selector first) * 10 | polars as a_times_10)
    | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(4)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(5)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(3), Value::test_int(6)],
                            ),
                            Column::new(
                                "a_times_10".to_string(),
                                vec![Value::test_int(10), Value::test_int(40)],
                            ),
                        ],
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
        let lazy = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
        command_lazy(plugin, engine, call, lazy)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    if let Some(name) = call.get_flag::<Spanned<String>>("name")? {
        return Err(ShellError::GenericError {
            error: "Flag 'name' is unsupported for lazy dataframes. Please use the `polars as` expression to name a column".into(),
            msg: "".into(),
            span: Some(name.span),
            help: Some("Use a `polars as` expression to name a column".into()),
            inner: vec![],
        });
    }

    let vals: Vec<Value> = call.rest(0)?;
    let value = Value::list(vals, call.head);
    let expressions = NuExpression::extract_exprs(plugin, value)?;
    let lazy: NuLazyFrame = lazy.to_polars().with_columns(&expressions).into();
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&WithColumn)
    }
}
