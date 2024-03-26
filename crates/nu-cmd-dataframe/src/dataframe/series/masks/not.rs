use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;
use polars::prelude::IntoSeries;

use std::ops::Not;

#[derive(Clone)]
pub struct NotSeries;

impl Command for NotSeries {
    fn name(&self) -> &str {
        "dfr not"
    }

    fn usage(&self) -> &str {
        "Inverts boolean mask."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Inverts boolean mask",
            example: "[true false true] | dfr into-df | dfr not",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_bool(false),
                            Value::test_bool(true),
                            Value::test_bool(false),
                        ],
                    )],
                    None,
                )
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
        let df = NuDataFrame::try_from_pipeline(input, call.head)?;
        command(engine_state, stack, call, df)
    }
}

fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let series = df.as_series(call.head)?;

    let bool = series.bool().map_err(|e| ShellError::GenericError {
        error: "Error inverting mask".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = bool.not();

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(NotSeries {})])
    }
}
