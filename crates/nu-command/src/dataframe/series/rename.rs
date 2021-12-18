use super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape,
};

#[derive(Clone)]
pub struct Rename;

impl Command for Rename {
    fn name(&self) -> &str {
        "dfr rename"
    }

    fn usage(&self) -> &str {
        "Renames a series"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("name", SyntaxShape::String, "new series name")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Renames a series",
            example: "[5 6 7 8] | dfr to-df | dfr rename new_name",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "new_name".to_string(),
                    vec![5.into(), 6.into(), 7.into(), 8.into()],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::unknown()),
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
    let name: String = call.req(engine_state, stack, 0)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let mut series = df.as_series(call.head)?;
    series.rename(&name);

    NuDataFrame::try_from_series(vec![series], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Rename {})])
    }
}
