use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};
use nu_engine::command_prelude::*;
use polars::chunked_array::ops::SortMultipleOptions;

#[derive(Clone)]
pub struct LazySortBy;

impl Command for LazySortBy {
    fn name(&self) -> &str {
        "dfr sort-by"
    }

    fn usage(&self) -> &str {
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
                example: "[[a b]; [6 2] [1 4] [4 1]] | dfr into-df | dfr sort-by a",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_int(1), Value::test_int(4), Value::test_int(6)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(4), Value::test_int(1), Value::test_int(2)],
                        ),
                    ], None)
                    .expect("simple df for test should not fail")
                    .into_value(FutureSpanId::test_data()),
                ),
            },
            Example {
                description: "Sort column using two columns",
                example:
                    "[[a b]; [6 2] [1 1] [1 4] [2 4]] | dfr into-df | dfr sort-by [a b] -r [false true]",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
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
                    ], None)
                    .expect("simple df for test should not fail")
                    .into_value(FutureSpanId::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::list(vals, call.head);
        let expressions = NuExpression::extract_exprs(value)?;
        let nulls_last = call.has_flag(engine_state, stack, "nulls-last")?;
        let maintain_order = call.has_flag(engine_state, stack, "maintain-order")?;

        let reverse: Option<Vec<bool>> = call.get_flag(engine_state, stack, "reverse")?;
        let reverse = match reverse {
            Some(list) => {
                if expressions.len() != list.len() {
                    let span = call
                        .get_flag::<Value>(engine_state, stack, "reverse")?
                        .expect("already checked and it exists")
                        .span();
                    return Err(ShellError::GenericError {
                        error: "Incorrect list size".into(),
                        msg: "Size doesn't match expression list".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    });
                } else {
                    list
                }
            }
            None => expressions.iter().map(|_| false).collect::<Vec<bool>>(),
        };

        let sort_options = SortMultipleOptions {
            descending: reverse,
            nulls_last,
            multithreaded: true,
            maintain_order,
        };

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let lazy = NuLazyFrame::new(
            lazy.from_eager,
            lazy.into_polars().sort_by_exprs(&expressions, sort_options),
        );

        Ok(PipelineData::Value(
            NuLazyFrame::into_value(lazy, call.head)?,
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazySortBy {})])
    }
}
