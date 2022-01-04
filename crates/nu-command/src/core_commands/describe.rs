use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Describe;

impl Command for Describe {
    fn name(&self) -> &str {
        "describe"
    }

    fn usage(&self) -> &str {
        "Describe the value(s) piped in."
    }

    fn signature(&self) -> Signature {
        Signature::build("describe").category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        if matches!(input, PipelineData::ByteStream(..)) {
            Ok(PipelineData::Value(
                Value::string("binary", call.head),
                None,
            ))
        } else {
            input.map(
                move |x| Value::String {
                    val: x.get_type().to_string(),
                    span: head,
                },
                engine_state.ctrlc.clone(),
            )
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Describe the type of a string",
            example: "'hello' | describe",
            result: Some(Value::test_string("string")),
        }]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Describe;
        use crate::test_examples;
        test_examples(Describe {})
    }
}
