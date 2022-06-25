use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use crate::dataframe::values::Column;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct SliceDF;

impl Command for SliceDF {
    fn name(&self) -> &str {
        "slice"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe from a slice of rows"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("offset", SyntaxShape::Int, "start of slice")
            .required("size", SyntaxShape::Int, "size of slice")
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Create new dataframe from a slice of the rows",
            example: "[[a b]; [1 2] [3 4]] | into df | slice 0 1",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("a".to_string(), vec![Value::test_int(1)]),
                    Column::new("b".to_string(), vec![Value::test_int(2)]),
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
    let offset: i64 = call.req(engine_state, stack, 0)?;
    let size: usize = call.req(engine_state, stack, 1)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let res = df.as_ref().slice(offset, size);

    Ok(PipelineData::Value(
        NuDataFrame::dataframe_into_value(res, call.head),
        None,
    ))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(SliceDF {})])
    }
}
