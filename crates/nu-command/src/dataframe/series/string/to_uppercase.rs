use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::{IntoSeries, Utf8NameSpaceImpl};

#[derive(Clone)]
pub struct ToUpperCase;

impl Command for ToUpperCase {
    fn name(&self) -> &str {
        "uppercase"
    }

    fn usage(&self) -> &str {
        "Uppercase the strings in the column"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["capitalize, caps, capital"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Modifies strings to uppercase",
            example: "[Abc aBc abC] | into df | uppercase",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "0".to_string(),
                    vec![
                        Value::test_string("ABC"),
                        Value::test_string("ABC"),
                        Value::test_string("ABC"),
                    ],
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

    let casted = series.utf8().map_err(|e| {
        ShellError::GenericError(
            "Error casting to string".into(),
            e.to_string(),
            Some(call.head),
            Some("The str-slice command can only be used with string columns".into()),
            Vec::new(),
        )
    })?;

    let mut res = casted.to_uppercase();
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
        test_dataframe(vec![Box::new(ToUpperCase {})])
    }
}
