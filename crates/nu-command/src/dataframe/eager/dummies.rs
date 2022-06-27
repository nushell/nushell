use super::super::values::{Column, NuDataFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::DataFrameOps;

#[derive(Clone)]
pub struct Dummies;

impl Command for Dummies {
    fn name(&self) -> &str {
        "dummies"
    }

    fn usage(&self) -> &str {
        "Creates a new dataframe with dummy variables"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create new dataframe with dummy variables from a dataframe",
                example: "[[a b]; [1 2] [3 4]] | into df | dummies",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a_1".to_string(),
                            vec![Value::test_int(1), Value::test_int(0)],
                        ),
                        Column::new(
                            "a_3".to_string(),
                            vec![Value::test_int(0), Value::test_int(1)],
                        ),
                        Column::new(
                            "b_2".to_string(),
                            vec![Value::test_int(1), Value::test_int(0)],
                        ),
                        Column::new(
                            "b_4".to_string(),
                            vec![Value::test_int(0), Value::test_int(1)],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create new dataframe with dummy variables from a series",
                example: "[1 2 2 3 3] | into df | dummies",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "0_1".to_string(),
                            vec![
                                Value::test_int(1),
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(0),
                            ],
                        ),
                        Column::new(
                            "0_2".to_string(),
                            vec![
                                Value::test_int(0),
                                Value::test_int(1),
                                Value::test_int(1),
                                Value::test_int(0),
                                Value::test_int(0),
                            ],
                        ),
                        Column::new(
                            "0_3".to_string(),
                            vec![
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(1),
                                Value::test_int(1),
                            ],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    df.as_ref()
        .to_dummies()
        .map_err(|e| {
            ShellError::GenericError(
                "Error calculating dummies".into(),
                e.to_string(),
                Some(call.head),
                Some("The only allowed column types for dummies are String or Int".into()),
                Vec::new(),
            )
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Dummies {})])
    }
}
