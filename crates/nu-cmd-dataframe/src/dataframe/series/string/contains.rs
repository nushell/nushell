use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;

use polars::prelude::{IntoSeries, StringNameSpaceImpl};

#[derive(Clone)]
pub struct Contains;

impl Command for Contains {
    fn name(&self) -> &str {
        "dfr contains"
    }

    fn usage(&self) -> &str {
        "Checks if a pattern is contained in a string."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "pattern",
                SyntaxShape::String,
                "Regex pattern to be searched",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns boolean indicating if pattern was found",
            example: "[abc acb acb] | dfr into-df | dfr contains ab",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_bool(true),
                            Value::test_bool(false),
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let pattern: String = call.req(engine_state, stack, 0)?;

    let series = df.as_series(call.head)?;
    let chunked = series.str().map_err(|e| ShellError::GenericError {
        error: "The contains command only with string columns".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let res = chunked
        .contains(&pattern, false)
        .map_err(|e| ShellError::GenericError {
            error: "Error searching in series".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Contains {})])
    }
}
