use super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct AllTrue;

impl Command for AllTrue {
    fn name(&self) -> &str {
        "all-true"
    }

    fn usage(&self) -> &str {
        "Returns true if all values are true"
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
                description: "Returns true if all values are true",
                example: "[true true true] | into df | all-true",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "all_true".to_string(),
                        vec![Value::test_bool(true)],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Checks the result from a comparison",
                example: r#"let s = ([5 6 2 8] | into df);
    let res = ($s > 9);
    $res | all-true"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "all_true".to_string(),
                        vec![Value::test_bool(false)],
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

    let series = df.as_series(call.head)?;
    let bool = series.bool().map_err(|_| {
        ShellError::GenericError(
            "Error converting to bool".into(),
            "all-false only works with series of type bool".into(),
            Some(call.head),
            None,
            Vec::new(),
        )
    })?;

    let value = Value::Bool {
        val: bool.all(),
        span: call.head,
    };

    NuDataFrame::try_from_columns(vec![Column::new("all_true".to_string(), vec![value])])
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(AllTrue {})])
    }
}
