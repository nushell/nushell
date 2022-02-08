use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature};

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
        Signature::build("ignore").category(Category::Core)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(PipelineData::new(call.head))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Ignore the output of an echo command",
            example: "echo done | ignore",
            result: None,
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
