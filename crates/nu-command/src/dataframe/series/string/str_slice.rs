use super::super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::{IntoSeries, Utf8NameSpaceImpl};

#[derive(Clone)]
pub struct StrSlice;

impl Command for StrSlice {
    fn name(&self) -> &str {
        "str-slice"
    }

    fn usage(&self) -> &str {
        "Slices the string from the start position until the selected length"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("start", SyntaxShape::Int, "start of slice")
            .named("length", SyntaxShape::Int, "optional length", Some('l'))
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates slices from the strings",
            example: "[abcded abc321 abc123] | into df | str-slice 1 -l 2",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "0".to_string(),
                    vec![
                        Value::test_string("bc"),
                        Value::test_string("bc"),
                        Value::test_string("bc"),
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
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let start: i64 = call.req(engine_state, stack, 0)?;

    let length: Option<i64> = call.get_flag(engine_state, stack, "length")?;
    let length = length.map(|v| v as u64);

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let series = df.as_series(call.head)?;

    let chunked = series.utf8().map_err(|e| {
        ShellError::GenericError(
            "Error casting to string".into(),
            e.to_string(),
            Some(call.head),
            Some("The str-slice command can only be used with string columns".into()),
            Vec::new(),
        )
    })?;

    let mut res = chunked.str_slice(start, length).map_err(|e| {
        ShellError::GenericError(
            "Error slicing series".into(),
            e.to_string(),
            Some(call.head),
            None,
            Vec::new(),
        )
    })?;
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
        test_dataframe(vec![Box::new(StrSlice {})])
    }
}
