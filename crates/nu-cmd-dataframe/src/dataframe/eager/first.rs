use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct FirstDF;

impl Command for FirstDF {
    fn name(&self) -> &str {
        "dfr first"
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows or create a first expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "rows",
                SyntaxShape::Int,
                "starting from the front, the number of rows to return",
            )
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first row of a dataframe",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr first",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(1)]),
                            Column::new("b".to_string(), vec![Value::test_int(2)]),
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Return the first two rows of a dataframe",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr first 2",
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
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a first expression from a column",
                example: "dfr col a | dfr first",
                result: None,
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
        let value = input.into_value(call.head);
        if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(value)?;
            command(engine_state, stack, call, df)
        } else {
            let expr = NuExpression::try_from_value(value)?;
            let expr: NuExpression = expr.into_polars().first().into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        }
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.opt(engine_state, stack, 0)?;
    let rows = rows.unwrap_or(1);

    let res = df.as_ref().head(Some(rows));
    Ok(PipelineData::Value(
        NuDataFrame::dataframe_into_value(res, call.head),
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::{build_test_engine_state, test_dataframe_example};
    use super::*;
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

    #[test]
    fn test_examples_dataframe() {
        let mut engine_state = build_test_engine_state(vec![Box::new(FirstDF {})]);
        test_dataframe_example(&mut engine_state, &FirstDF.examples()[0]);
        test_dataframe_example(&mut engine_state, &FirstDF.examples()[1]);
    }

    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(FirstDF {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &FirstDF.examples()[2]);
    }
}
