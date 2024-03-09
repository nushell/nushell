use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Span, Type, Value};

#[derive(Clone)]
pub struct Ignore;

impl Command for Ignore {
    fn name(&self) -> &str {
        "ignore"
    }

    fn usage(&self) -> &str {
        "Ignore the output of the previous command in the pipeline."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ignore")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .category(Category::Core)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["silent", "quiet", "out-null"]
    }

    fn is_const(&self) -> bool {
        true
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        input.drain()?;
        Ok(PipelineData::empty())
    }

    fn run_const(
        &self,
        _working_set: &StateWorkingSet,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        input.drain()?;
        Ok(PipelineData::empty())
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
