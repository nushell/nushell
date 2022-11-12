use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct Ignore;

impl Command for Ignore {
    fn name(&self) -> &str {
        "ignore"
    }

    fn usage(&self) -> &str {
        "Ignore the output of the previous command in the pipeline"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ignore")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .category(Category::Core)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["silent", "quiet", "out-null"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        input.into_value(call.head);
        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Ignore the output of an echo command",
            example: "echo done | ignore",
            result: Some(Value::nothing(Span::test_data())),
        }]
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Ignore;
        use crate::test_examples;
        test_examples(Ignore {})
    }
}
