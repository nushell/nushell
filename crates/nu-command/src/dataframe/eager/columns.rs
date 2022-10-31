use super::super::values::NuDataFrame;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

#[derive(Clone)]
pub struct ColumnsDF;

impl Command for ColumnsDF {
    fn name(&self) -> &str {
        "columns"
    }

    fn usage(&self) -> &str {
        "Show dataframe columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Any)
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Dataframe columns",
            example: "[[a b]; [1 2] [3 4]] | into df | columns",
            result: Some(Value::List {
                vals: vec![
                    Value::String {
                        val: "a".into(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "b".into(),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
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
        .map(|v| Value::String {
            val: v.to_string(),
            span: call.head,
        })
        .collect();

    let names = Value::List {
        vals: names,
        span: call.head,
    };

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
