use crate::dataframe::values::{utils::DEFAULT_ROWS, Column, NuDataFrame, NuExpression};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct LastDF;

impl Command for LastDF {
    fn name(&self) -> &str {
        "dfr last"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe with tail rows or creates a last expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional("rows", SyntaxShape::Int, "Number of rows for tail")
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
                description: "Create new dataframe with last rows",
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr last 1",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(3)]),
                            Column::new("b".to_string(), vec![Value::test_int(4)]),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a last expression from a column",
                example: "dfr col a | dfr last",
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
            let expr: NuExpression = expr.into_polars().last().into();

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
    let rows = rows.unwrap_or(DEFAULT_ROWS);

    let res = df.as_ref().tail(Some(rows));
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
        let mut engine_state = build_test_engine_state(vec![Box::new(LastDF {})]);
        test_dataframe_example(&mut engine_state, &LastDF.examples()[0]);
    }

    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(LastDF {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &LastDF.examples()[1]);
    }
}
