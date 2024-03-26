use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;

use polars::prelude::{IntoSeries, StringNameSpaceImpl};

#[derive(Clone)]
pub struct Concatenate;

impl Command for Concatenate {
    fn name(&self) -> &str {
        "dfr concatenate"
    }

    fn usage(&self) -> &str {
        "Concatenates strings with other array."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "other",
                SyntaxShape::Any,
                "Other array with string to be concatenated",
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Concatenate string",
            example: r#"let other = ([za xs cd] | dfr into-df);
    [abc abc abc] | dfr into-df | dfr concatenate $other"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::test_string("abcza"),
                            Value::test_string("abcxs"),
                            Value::test_string("abccd"),
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

    let other: Value = call.req(engine_state, stack, 0)?;
    let other_span = other.span();
    let other_df = NuDataFrame::try_from_value(other)?;

    let other_series = other_df.as_series(other_span)?;
    let other_chunked = other_series.str().map_err(|e| ShellError::GenericError {
        error: "The concatenate only with string columns".into(),
        msg: e.to_string(),
        span: Some(other_span),
        help: None,
        inner: vec![],
    })?;

    let series = df.as_series(call.head)?;
    let chunked = series.str().map_err(|e| ShellError::GenericError {
        error: "The concatenate only with string columns".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let mut res = chunked.concat(other_chunked);

    res.rename(series.name());

    NuDataFrame::try_from_series(vec![res.into_series()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Concatenate {})])
    }
}
