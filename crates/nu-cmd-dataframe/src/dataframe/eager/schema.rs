use super::super::values::{Column, NuDataFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct SchemaDF;

impl Command for SchemaDF {
    fn name(&self) -> &str {
        "dfr schema"
    }

    fn usage(&self) -> &str {
        "Show schema for dataframe"
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
        //todo 
        vec![]
    //     vec![Example {
    //         description: "Dataframe dtypes",
    //         example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr dtypes",
    //         result: Some(
    //             NuDataFrame::try_from_columns(vec![
    //                 Column::new(
    //                     "column".to_string(),
    //                     vec![Value::test_string("a"), Value::test_string("b")],
    //                 ),
    //                 Column::new(
    //                     "dtype".to_string(),
    //                     vec![Value::test_string("i64"), Value::test_string("i64")],
    //                 ),
    //             ])
    //             .expect("simple df for test should not fail")
    //             .into_value(Span::test_data()),
    //         ),
    //     }]
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
    let schema = df.schema();
    let value: Value = schema.into();
    Ok(PipelineData::Value(value, None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(SchemaDF {})])
    }
}
