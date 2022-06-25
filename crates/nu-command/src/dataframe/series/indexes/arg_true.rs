use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct ArgTrue;

impl Command for ArgTrue {
    fn name(&self) -> &str {
        "arg-true"
    }

    fn usage(&self) -> &str {
        "Returns indexes where values are true"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns indexes where values are true",
            example: "[false true false] | into df | arg-true",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "arg_true".to_string(),
                    vec![Value::test_int(1)],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
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

    let mut res = bool.arg_true().into_series();
    res.rename("arg_true");

    NuDataFrame::try_from_series(vec![res], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ArgTrue {})])
    }
}
