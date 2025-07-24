use crate::values::NuLazyFrame;
use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::CustomValueSupport,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::chunked_array::ops::SortMultipleOptions;

#[derive(Clone)]
pub struct LazySortBy;

impl PluginCommand for LazySortBy {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars sort-by"
    }

    fn description(&self) -> &str {
        "Sorts a lazy dataframe based on expression(s)."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "sort expression",
                SyntaxShape::Any,
                "sort expression for the dataframe",
            )
            .named(
                "reverse",
                SyntaxShape::List(Box::new(SyntaxShape::Boolean)),
                "Reverse sorting. Default is false",
                Some('r'),
            )
            .switch(
                "nulls-last",
                "nulls are shown last in the dataframe",
                Some('n'),
            )
            .switch("maintain-order", "Maintains order during sort", Some('m'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Sort dataframe by one column",
                example: "[[a b]; [6 2] [1 4] [4 1]] | polars into-df | polars sort-by a",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(4), Value::test_int(6)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(4), Value::test_int(1), Value::test_int(2)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Sort column using two columns",
                example: "[[a b]; [6 2] [1 1] [1 4] [2 4]] | polars into-df | polars sort-by [a b] -r [false true]",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![
                                    Value::test_int(1),
                                    Value::test_int(1),
                                    Value::test_int(2),
                                    Value::test_int(6),
                                ],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![
                                    Value::test_int(4),
                                    Value::test_int(1),
                                    Value::test_int(4),
                                    Value::test_int(2),
                                ],
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
        let vals: Vec<Value> = call.rest(0)?;
        let expr_value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(plugin, expr_value)?;
        let nulls_last = call.has_flag("nulls-last")?;
        let maintain_order = call.has_flag("maintain-order")?;

        let reverse: Option<Vec<bool>> = call.get_flag("reverse")?;
        let reverse = match reverse {
            Some(list) => {
                if expressions.len() != list.len() {
                    let span = call
                        .get_flag::<Value>("reverse")?
                        .expect("already checked and it exists")
                        .span();
                    Err(ShellError::GenericError {
                        error: "Incorrect list size".into(),
                        msg: "Size doesn't match expression list".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    })?
                } else {
                    list
                }
            }
            None => expressions.iter().map(|_| false).collect::<Vec<bool>>(),
        };

        let sort_options = SortMultipleOptions {
            descending: reverse,
            nulls_last: vec![nulls_last],
            multithreaded: true,
            maintain_order,
            // Applying a limit here will result in a panic
            // it is not supported by polars in this context
            limit: None,
        };

        let pipeline_value = input.into_value(call.head)?;
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &pipeline_value)?;
        let lazy = NuLazyFrame::new(
            lazy.from_eager,
            lazy.to_polars().sort_by_exprs(&expressions, sort_options),
        );
        lazy.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&LazySortBy)
    }
}
