use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct RenameDF;

impl Command for RenameDF {
    fn name(&self) -> &str {
        "dfr rename-col"
    }

    fn usage(&self) -> &str {
        "rename a dataframe column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("from", SyntaxShape::String, "column name to be renamed")
            .required("to", SyntaxShape::String, "new column name")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Renames a dataframe column",
            example: "[[a b]; [1 2] [3 4]] | dfr to-df | dfr rename-col a a-new",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a-new".to_string(),
                        vec![Value::test_int(1), Value::test_int(3)],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(4)],
                    ),
                ])
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
    let from: String = call.req(engine_state, stack, 0)?;
    let to: String = call.req(engine_state, stack, 1)?;

    let mut df = NuDataFrame::try_from_pipeline(input, call.head)?;

    df.as_mut()
        .rename(&from, &to)
        .map_err(|e| {
            ShellError::SpannedLabeledError("Error renaming".into(), e.to_string(), call.head)
        })
        .map(|df| {
            PipelineData::Value(
                NuDataFrame::dataframe_into_value(df.clone(), call.head),
                None,
            )
        })
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(RenameDF {})])
    }
}
