use crate::dataframe::values::{Column, NuDataFrame, NuExpression};
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct NUnique;

impl Command for NUnique {
    fn name(&self) -> &str {
        "dfr n-unique"
    }

    fn usage(&self) -> &str {
        "Counts unique values."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
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
                description: "Counts unique values",
                example: "[1 1 2 2 3 3 4] | dfr into-df | dfr n-unique",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "count_unique".to_string(),
                            vec![Value::test_int(4)],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a is n-unique expression from a column",
                example: "dfr col a | dfr n-unique",
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
            let expr: NuExpression = expr.into_polars().n_unique().into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        }
    }
}

fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let res = df
        .as_series(call.head)?
        .n_unique()
        .map_err(|e| ShellError::GenericError {
            error: "Error counting unique values".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let value = Value::int(res as i64, call.head);

    NuDataFrame::try_from_columns(
        vec![Column::new("count_unique".to_string(), vec![value])],
        None,
    )
    .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::{build_test_engine_state, test_dataframe_example};
    use super::*;
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

    #[test]
    fn test_examples_dataframe() {
        let mut engine_state = build_test_engine_state(vec![Box::new(NUnique {})]);
        test_dataframe_example(&mut engine_state, &NUnique.examples()[0]);
    }

    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(NUnique {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &NUnique.examples()[1]);
    }
}
