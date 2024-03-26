use crate::dataframe::values::NuDataFrame;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ColumnsDF;

impl Command for ColumnsDF {
    fn name(&self) -> &str {
        "dfr columns"
    }

    fn usage(&self) -> &str {
        "Show dataframe columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Custom("dataframe".into()), Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Dataframe columns",
            example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr columns",
            result: Some(Value::list(
                vec![Value::test_string("a"), Value::test_string("b")],
                Span::test_data(),
            )),
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

    let names: Vec<Value> = df
        .as_ref()
        .get_column_names()
        .iter()
        .map(|v| Value::string(*v, call.head))
        .collect();

    let names = Value::list(names, call.head);

    Ok(PipelineData::Value(names, None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ColumnsDF {})])
    }
}
