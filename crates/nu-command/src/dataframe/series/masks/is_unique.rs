use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsUnique;

impl Command for IsUnique {
    fn name(&self) -> &str {
        "is-unique"
    }

    fn usage(&self) -> &str {
        "Creates mask indicating unique values"
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
                description: "Create mask indicating unique values",
                example: "[5 6 6 6 8 8 8] | into df | is-unique",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "is_unique".to_string(),
                        vec![
                            Value::test_bool(true),
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(false),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create mask indicating duplicated rows in a dataframe",
                example: "[[a, b]; [1 2] [1 2] [3 3] [3 3] [1 1]] | into df | is-unique",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "is_unique".to_string(),
                        vec![
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(false),
                            Value::test_bool(true),
                        ],
                    )])
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

    let mut res = df
        .as_ref()
        .is_unique()
        .map_err(|e| {
            ShellError::GenericError(
                "Error finding unique values".into(),
                e.to_string(),
                Some(call.head),
                None,
                Vec::new(),
            )
        })?
        .into_series();

    res.rename("is_unique");

    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(IsUnique {})])
    }
}
